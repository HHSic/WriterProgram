import { createRoot } from 'react-dom/client';
import '@fontsource/ibm-plex-sans-kr/400.css';
import '@fontsource/ibm-plex-sans-kr/600.css';
import '@fontsource/noto-serif-kr/400.css';
import '@fontsource/noto-serif-kr/700.css';
import '@fontsource/gowun-batang/400.css';
import '@fontsource/gowun-batang/700.css';
import '@fontsource/nanum-myeongjo/400.css';
import '@fontsource/nanum-myeongjo/700.css';
import './styles.css';
import { App } from './App';
import * as store from './store';

// Lets browser checks look at the app state during development.
if (import.meta.env.DEV) Object.assign(window, { __app: store.useApp, __store: store });

createRoot(document.getElementById('root')!).render(<App />);
