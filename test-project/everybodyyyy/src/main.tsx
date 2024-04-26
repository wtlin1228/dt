import React from 'react'
import ReactDOM from 'react-dom/client'
import { App1, App2, App3 } from './App.tsx'
import './index.css'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App1 />
    <App2 />
    <App3 />
  </React.StrictMode>
)
