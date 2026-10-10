-- Run once in the Supabase SQL editor. No admin credentials belong in the app.
create table if not exists public.writer_settings (
  user_id uuid primary key references auth.users(id) on delete cascade,
  revision bigint not null default 0 check (revision >= 0),
  settings jsonb not null default '{}'::jsonb,
  updated_at timestamptz not null default now()
);
alter table public.writer_settings enable row level security;
drop policy if exists "read own writer settings" on public.writer_settings;
create policy "read own writer settings" on public.writer_settings for select
  to authenticated using ((select auth.uid()) = user_id);
revoke all on public.writer_settings from anon, authenticated;
grant select on public.writer_settings to authenticated;

create or replace function public.writer_settings_valid(patch jsonb)
returns boolean language plpgsql immutable set search_path = '' as $$
declare section text; fields jsonb; name text; value jsonb; kind text; word text;
begin
  if patch is null or jsonb_typeof(patch) <> 'object' or octet_length(patch::text) > 32768 then return false; end if;
  for section, fields in select * from jsonb_each(patch) loop
    if section not in ('view','ribbon','book') or jsonb_typeof(fields) <> 'object' then return false; end if;
    for name, value in select * from jsonb_each(fields) loop
      kind := jsonb_typeof(value); word := value #>> '{}';
      if section = 'view' then
        if name in ('showMarks','ruler','pages','focusDim','typewriter','autoType') then
          if kind <> 'boolean' then return false; end if;
        elsif name in ('fontSize','lineHeight','letterSpacing','paragraphGap','indent','width','readRate') then
          if kind <> 'number' then return false; end if;
          if name = 'fontSize' and word::numeric < 1 then return false; end if;
          if name in ('lineHeight','width','readRate') and word::numeric <= 0 then return false; end if;
        elsif kind <> 'string' then return false;
        elsif name = 'theme' then if word not in ('system','light','dark') then return false; end if;
        elsif name = 'font' then if word not in ('noto-serif','gowun-batang','nanum-myeongjo','sans') then return false; end if;
        elsif name = 'palette' then if word not in ('beige','white','gray','green','sky','pink','custom') then return false; end if;
        elsif name = 'accent' then if word not in ('auto','vermilion','blue','green','violet','ink') then return false; end if;
        elsif name = 'toolbar' then if word not in ('auto','always','never') then return false; end if;
        elsif name = 'quoteStyle' then if word not in ('curly','corner') then return false; end if;
        elsif name = 'customColor' then if word !~ '^#[0-9a-fA-F]{6}$' then return false; end if;
        else return false;
        end if;
      elsif section = 'ribbon' then
        if name = 'tab' then if kind <> 'string' or word not in ('편집','서식','보기','쪽','검토') then return false; end if;
        elsif name = 'collapsed' then if kind <> 'boolean' then return false; end if;
        else return false;
        end if;
      else
        if name = 'theme' then if kind <> 'string' or word not in ('paper','light','dark') then return false; end if;
        elsif name in ('fontSize','pageWidth','pageHeight') then
          if kind <> 'number' then return false; end if;
          if word::numeric < (case when name = 'fontSize' then 1 else 20 end) then return false; end if;
        else return false;
        end if;
      end if;
    end loop;
  end loop;
  return true;
end;
$$;
revoke all on function public.writer_settings_valid(jsonb) from public, anon, authenticated;
alter table public.writer_settings drop constraint if exists writer_settings_portable;
alter table public.writer_settings add constraint writer_settings_portable check (public.writer_settings_valid(settings));

create or replace function public.writer_settings_read()
returns jsonb language plpgsql security definer set search_path = '' as $$
declare result jsonb;
begin
  if auth.uid() is null then raise exception 'Authentication required' using errcode = '42501'; end if;
  select jsonb_build_object('revision',revision,'settings',settings,'updatedAt',updated_at)
    into result from public.writer_settings where user_id = auth.uid();
  return coalesce(result,jsonb_build_object('revision',0,'settings','{}'::jsonb,'updatedAt',null));
end;
$$;

create or replace function public.writer_settings_write(expected_revision bigint, patch jsonb)
returns jsonb language plpgsql security definer set search_path = '' as $$
declare uid uuid := auth.uid(); row public.writer_settings%rowtype; merged jsonb; section text; fields jsonb;
begin
  if uid is null then raise exception 'Authentication required' using errcode = '42501'; end if;
  if expected_revision is null or expected_revision < 0 or not public.writer_settings_valid(patch) then
    raise exception 'Invalid portable settings' using errcode = '22023';
  end if;
  insert into public.writer_settings(user_id) values(uid) on conflict do nothing;
  select * into row from public.writer_settings where user_id = uid for update;
  if row.revision <> expected_revision then
    return jsonb_build_object('conflict',true,'snapshot',jsonb_build_object('revision',row.revision,'settings',row.settings,'updatedAt',row.updated_at));
  end if;
  merged := row.settings;
  for section,fields in select * from jsonb_each(patch) loop
    merged := jsonb_set(merged,array[section],coalesce(merged->section,'{}'::jsonb)||fields,true);
  end loop;
  update public.writer_settings set settings=merged,revision=row.revision+1,updated_at=now()
    where user_id=uid returning * into row;
  return jsonb_build_object('conflict',false,'snapshot',jsonb_build_object('revision',row.revision,'settings',row.settings,'updatedAt',row.updated_at));
end;
$$;
revoke all on function public.writer_settings_read() from public, anon;
revoke all on function public.writer_settings_write(bigint,jsonb) from public, anon;
grant execute on function public.writer_settings_read() to authenticated;
grant execute on function public.writer_settings_write(bigint,jsonb) to authenticated;
