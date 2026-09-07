// Deterministic IPC fixture shared by renderer profiling and native-boundary tests.
export function installFixture() {
  const image = (index) => `data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="640" height="900"><rect width="640" height="900" fill="#242424"/><text x="50" y="100" fill="white">Page ${index + 1}</text></svg>`)}`;
  const gallery = (gid) => ({ gid, token: 'fixture', title: `Profile gallery ${gid}`, titleJpn: null, category: 'Manga', uploader: 'fixture', posted: 1700000000, pages: 240, rating: 4, ratedCount: 1, favorited: 0, language: 'English', thumb: image(gid), size: 0, torrentCount: 0, fileCount: 240 });
  window.__profileCalls = {};
  let nextId = 1;
  const callbacks = new Map();
  const listeners = new Map();
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener: (_event, id) => { callbacks.delete(listeners.get(id)?.handler); listeners.delete(id); },
  };
  window.__profileEmit = (event) => {
    for (const [id, listener] of listeners) {
      if (listener.event === event) callbacks.get(listener.handler)?.({event, id, payload: null});
    }
  };
  window.__TAURI_INTERNALS__ = {
    transformCallback: (callback) => { const id = nextId++; callbacks.set(id, callback); return id; },
    unregisterCallback: (id) => { callbacks.delete(id); },
    invoke: async (command, args = {}) => {
      window.__profileCalls[command] = (window.__profileCalls[command] ?? 0) + 1;
      switch (command) {
        case 'plugin:store|load': return 1;
        case 'plugin:store|get': return [JSON.parse(localStorage.getItem('profile-settings') ?? '{}'), true];
        case 'plugin:store|set': localStorage.setItem('profile-settings', JSON.stringify(args.value)); return;
        case 'plugin:store|save': return;
        case 'plugin:event|listen': { const id = nextId++; listeners.set(id, args); return id; }
        case 'plugin:event|unlisten': return;
        case 'get_gallery_list': {
          const page = args.params.page;
          return { galleries: Array.from({length:25}, (_, i) => gallery(page * 25 + i + 1)), total: 1000, pages:40, nextPage: page < 39 ? page + 1 : null };
        }
        case 'get_image': return args.url;
        case 'list_downloads': return [];
        case 'download_labels': return [];
        case 'get_download_root': return '/profile-downloads';
        case 'check_sign_in': return false;
        case 'get_quick_searches': case 'search_history': return [];
        case 'get_downloaded_gallery': return {gid:1, token:'fixture', title:'Profile reader', pages:240, availablePages:Array.from({length:240},(_,i)=>i)};
        case 'get_downloaded_page': return {index:args.index, url:image(args.index), width:640, height:900};
        default: throw new Error(`Unexpected profiling IPC command: ${command}`);
      }
    },
  };
}
