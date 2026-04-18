// fake glue
export async function init() {
    const url = new URL('solobase_web_bg.wasm', import.meta.url);
    return fetch(url);
}
