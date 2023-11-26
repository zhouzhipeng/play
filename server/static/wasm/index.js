
window.addEventListener('load', async () => {
    let client =  await import('./pkg/client.js');
    await client.default('./pkg/client_bg.wasm');
    window.wasm = client;


    async function run() {

        const audioUrl = 'https://www.learningcontainer.com/wp-content/uploads/2020/02/Kalimba.mp3?_=1'; // Replace with the URL of your audio file
        const canvasId = 'myCanvas'; // Replace with the ID of your canvas element

        try {
            await window.wasm.run_music_visualizer(audioUrl, canvasId);
            console.log('Audio started playing with visualization.');
        } catch (error) {
            console.error('Error playing audio with visualization:', error);
        }
    }

    await run();
});
