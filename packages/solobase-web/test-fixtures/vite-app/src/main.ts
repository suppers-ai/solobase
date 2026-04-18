import { registerWithUpdates } from 'solobase-web';

registerWithUpdates('/sw.js').then((handle) => {
  console.log('registered', handle.registration);
});
