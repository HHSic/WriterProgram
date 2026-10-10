import {beforeEach,expect,it,vi} from 'vitest';
import type {Overview,SnapshotInfo} from '../api/types';
import {useApp} from './state';
import {restoreRecord} from './records';
const mock=vi.hoisted(()=>({save:vi.fn(),open:vi.fn(),patch:vi.fn(),error:vi.fn(),restore:vi.fn(),load:vi.fn()}));
vi.mock('../api',()=>({api:{snapshotRestore:mock.restore,docLoad:mock.load}}));
vi.mock('./ui',()=>({saveEverything:mock.save,openDialog:mock.open,patchSummary:mock.patch,toastError:mock.error,findDoc:(ov:Overview,id:string)=>{const doc=ov.parts.flatMap(p=>p.docs).find(d=>d.id===id);return doc?{doc}:null;}}));
const record={id:'record',at:'2026-10-10T00:00:00Z',name:'초고'} as SnapshotInfo;
beforeEach(()=>{
  vi.clearAllMocks();
  useApp.setState({overview:{root:'/test/project',parts:[{docs:[{id:'chapter',locked:false}]}]} as unknown as Overview,conflicts:{},docVersion:0,recordsVersion:0});
  mock.save.mockResolvedValue(true);mock.restore.mockResolvedValue({id:'before-restore'});mock.load.mockResolvedValue({counts:{withSpaces:15,withoutSpaces:12}});
});
it('waits for saving before restoring and refreshes the kept document',async()=>{
  const order:string[]=[];
  mock.save.mockImplementation(async()=>{order.push('save');return true;});
  mock.restore.mockImplementation(async()=>{order.push('restore');return {};});
  restoreRecord('chapter',record);await mock.open.mock.calls[0][0].onConfirm();
  expect(order).toEqual(['save','restore']);expect(mock.restore).toHaveBeenCalledWith('/test/project','chapter','record');
  expect(mock.patch).toHaveBeenCalledWith('chapter',{counts:{withSpaces:15,withoutSpaces:12}});
  expect(useApp.getState().docVersion).toBe(1);expect(useApp.getState().recordsVersion).toBe(1);
});
it('keeps the manuscript when saving fails',async()=>{
  mock.save.mockResolvedValue(false);restoreRecord('chapter',record);await mock.open.mock.calls[0][0].onConfirm();
  expect(mock.restore).not.toHaveBeenCalled();
});
it('does not restore after switching projects during a pending save',async()=>{
  mock.save.mockImplementation(async()=>{useApp.setState({overview:{root:'/other/project',parts:[]} as unknown as Overview});return true;});
  restoreRecord('chapter',record);await mock.open.mock.calls[0][0].onConfirm();expect(mock.restore).not.toHaveBeenCalled();
});
it('does not restore a locked or conflicted document',()=>{
  useApp.getState().overview!.parts[0].docs[0].locked=true;restoreRecord('chapter',record);expect(mock.open).not.toHaveBeenCalled();
  useApp.getState().overview!.parts[0].docs[0].locked=false;useApp.setState({conflicts:{chapter:{record:null,at:'now'}}});
  restoreRecord('chapter',record);expect(mock.open).not.toHaveBeenCalled();
});
