// @vitest-environment node
import migration from '../../../supabase/migrations/202610100001_personal_settings.sql?raw';
import { PGlite } from '@electric-sql/pglite';
import { afterAll, beforeAll, beforeEach, expect, it } from 'vitest';
import type { SettingsSnapshot, SettingsWrite } from '../api/types';

let db:PGlite;
const A='10000000-0000-0000-0000-000000000001', B='20000000-0000-0000-0000-000000000002';
beforeAll(async()=>{
  db=new PGlite();
  await db.exec("create schema auth; create role anon; create role authenticated; create table auth.users(id uuid primary key); create function auth.uid() returns uuid language sql stable as $$select nullif(current_setting('request.jwt.claim.sub',true),'')::uuid$$; grant usage on schema auth to anon,authenticated;");
  await db.query('insert into auth.users values ($1),($2)',[A,B]);
  await db.exec(migration);
},30_000);
afterAll(async()=>{await db?.close();});
beforeEach(async()=>{await db.exec('reset role; delete from public.writer_settings;');await asUser(A);});
async function asUser(id:string|null) {
  await db.exec('reset role');
  await db.query("select set_config('request.jwt.claim.sub',$1,false)",[id||'']);
  await db.exec('set role authenticated');
}
async function read() {return (await db.query<{result:SettingsSnapshot}>('select public.writer_settings_read() as result')).rows[0].result;}
async function put(revision:number,patch:unknown) {
  return (await db.query<{result:SettingsWrite}>('select public.writer_settings_write($1,$2::jsonb) as result',[revision,JSON.stringify(patch)])).rows[0].result;
}
it('isolates settings by the authenticated subject, including direct table reads',async()=>{
  await put(0,{view:{fontSize:1200,palette:'sky'},ribbon:{tab:'보기'}});
  await asUser(B);
  expect(await read()).toMatchObject({revision:0,settings:{}});
  expect((await db.query('select * from public.writer_settings')).rows).toEqual([]);
  await put(0,{view:{fontSize:22}});
  await asUser(A);
  expect((await read()).settings.view).toEqual({fontSize:1200,palette:'sky'});
  await expect(db.query("update public.writer_settings set settings='{}'")).rejects.toThrow(/permission denied/i);
});
it('rejects stale writes and merges only changed fields on retry',async()=>{
  await put(0,{view:{fontSize:18,theme:'light'},book:{pageWidth:148,pageHeight:210}});
  const first=await put(1,{view:{fontSize:25}});
  const stale=await put(1,{view:{theme:'dark'}});
  expect(first.conflict).toBe(false);
  expect(stale).toMatchObject({conflict:true,snapshot:{revision:2,settings:{view:{fontSize:25,theme:'light'}}}});
  await put(stale.snapshot.revision,{view:{theme:'dark'}});
  expect((await read()).settings).toEqual({view:{fontSize:25,theme:'dark'},book:{pageWidth:148,pageHeight:210}});
});
it('rejects credentials, device settings and malformed preferences',async()=>{
  for(const patch of [{password:'secret'},{view:{apiKey:'secret'}},{view:{readVoice:'device voice'}},{book:{device:'phone'}},{ribbon:{collapsed:'true'}},{view:{fontSize:0}},{view:{font:'unknown'}},null]) {
    await expect(put(0,patch)).rejects.toThrow(/Invalid portable settings/i);
  }
  await expect(db.query("select public.writer_settings_write(null,'{}')")).rejects.toThrow(/Invalid portable settings/i);
  expect((await read()).revision).toBe(0);
});
it('requires authentication and does not expose RPCs to anonymous requests',async()=>{
  await asUser(null);
  await expect(read()).rejects.toThrow(/Authentication required/i);
  await db.exec('reset role; set role anon');
  await expect(read()).rejects.toThrow(/permission denied/i);
  await expect(put(0,{})).rejects.toThrow(/permission denied/i);
  await expect(db.query('select * from public.writer_settings')).rejects.toThrow(/permission denied/i);
});
