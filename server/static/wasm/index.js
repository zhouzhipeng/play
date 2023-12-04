
window.addEventListener('load', async () => {
    let client =  await import('./pkg/client.js');
    await client.default('./pkg/client_bg.wasm');
    window.wasm = client;
    console.log(client.greet("zzp"))
});
