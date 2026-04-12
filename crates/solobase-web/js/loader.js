async function boot() {
    const status = document.getElementById('status');
    if (!('serviceWorker' in navigator)) {
        status.textContent = 'Service Workers not supported in this browser.';
        return;
    }
    try {
        status.textContent = 'Registering Service Worker...';
        const registration = await navigator.serviceWorker.register('/sw.js', {
            type: 'module',
            scope: '/',
        });
        const sw = registration.installing || registration.waiting || registration.active;
        if (sw && sw.state !== 'activated') {
            await new Promise((resolve) => {
                sw.addEventListener('statechange', () => {
                    if (sw.state === 'activated') resolve();
                });
                if (sw.state === 'activated') resolve();
            });
        }
        if (!navigator.serviceWorker.controller) {
            status.textContent = 'First-time setup complete. Loading Solobase...';
            window.location.reload();
            return;
        }
        status.textContent = 'Loading Solobase...';
        window.location.href = '/b/system/';
    } catch (error) {
        status.textContent = 'Error: ' + error.message;
        console.error('[solobase-web] Boot error:', error);
    }
}
boot();
