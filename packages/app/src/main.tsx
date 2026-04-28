/* @refresh reload */
import { render } from 'solid-js/web';
import { App } from './App';
import './index.css';

const root = document.getElementById('root');

if (!root) {
  throw new Error(
    'Root element #root not found. Check that index.html includes <div id="root"></div>.',
  );
}

render(() => <App />, root);

// Phase 2.5 — once the Solid root has rendered, swap out the
// pre-hydration skeleton. Defer to the next animation frame so the
// browser commits at least one paint of the skeleton first.
const skeleton = document.getElementById('skeleton');
if (skeleton) {
  requestAnimationFrame(() => skeleton.remove());
}
