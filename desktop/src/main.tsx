import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { initConsoleBranding } from './utils/consoleBranding';
import './index.css';

initConsoleBranding();

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
