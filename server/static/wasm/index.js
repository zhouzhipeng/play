import init from './pkg/client.js';

window.addEventListener('load', async () => {
    await init('./pkg/client_bg.wasm');
});
