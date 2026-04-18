import { initialize, handleRequest } from 'solobase-web/worker';

self.addEventListener('install', (event: any) => {
  event.waitUntil(initialize());
});

self.addEventListener('fetch', (event: any) => {
  event.respondWith(handleRequest(event.request));
});
