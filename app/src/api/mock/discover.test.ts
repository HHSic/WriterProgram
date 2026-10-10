import { expect,it } from 'vitest';
import { suggestNames } from './discover';
it('suggests speech subjects, honorifics and travel destinations locally',()=>{
 const names=suggestNames('윤서하가 말했다. 엘리시아는 고개를 끄덕였다. 민준 씨는 웃었다. 그녀는 물었다. 바람이 속삭였다. 둘은 달빛 서점에서 만났다. 지하 서고에 들어갔다. 서울로 향했다. 마음에 들어왔다.');
 for(const name of ['윤서하','엘리시아','민준'])expect(names.get(name)).toBe('person');
 for(const name of ['달빛 서점','지하 서고','서울'])expect(names.get(name)).toBe('place');
 for(const name of ['그녀','바람','마음'])expect(names.has(name)).toBe(false);
});
