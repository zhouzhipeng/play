import init from './pkg/client.js';

window.addEventListener('load', async () => {
    window.wasm = await init('./pkg/client_bg.wasm');
});
