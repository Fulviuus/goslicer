// Wait for Tauri to be ready
document.addEventListener('DOMContentLoaded', () => {
  init();
});

async function init() {
  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;

  const dropZone = document.getElementById('drop-zone');
  const processing = document.getElementById('processing');
  const results = document.getElementById('results');
  const layersGrid = document.getElementById('layers-grid');
  const resultsTitle = document.getElementById('results-title');
  const openFolderBtn = document.getElementById('open-folder-btn');
  const resetBtn = document.getElementById('reset-btn');

  let currentOutputDir = '';

  // --- Drop Zone Click → open native file picker ---
  dropZone.addEventListener('click', async () => {
    try {
      const filePath = await invoke('pick_psd_file');
      if (filePath) {
        processFile(filePath);
      }
    } catch (err) {
      console.error('File dialog error:', err);
    }
  });

  // --- Tauri v2 file drop events ---
  listen('tauri://drag-drop', (event) => {
    const paths = event.payload.paths;
    if (paths && paths.length > 0) {
      const file = paths[0];
      if (file.toLowerCase().endsWith('.psd')) {
        processFile(file);
      }
    }
  });

  listen('tauri://drag-enter', () => {
    dropZone.classList.add('drag-over');
  });

  listen('tauri://drag-leave', () => {
    dropZone.classList.remove('drag-over');
  });

  // --- Process File ---
  async function processFile(filePath) {
    dropZone.classList.add('hidden');
    results.classList.add('hidden');
    processing.classList.remove('hidden');
    layersGrid.innerHTML = '';

    try {
      const result = await invoke('process_psd', { filePath });
      currentOutputDir = result.output_dir;

      processing.classList.add('hidden');
      results.classList.remove('hidden');

      const count = result.layers.length;
      resultsTitle.textContent = `${result.file_name} — ${count} layer${count !== 1 ? 's' : ''} extracted`;

      result.layers.forEach((layer, index) => {
        const card = createLayerCard(layer, index);
        layersGrid.appendChild(card);
      });
    } catch (err) {
      processing.classList.add('hidden');
      dropZone.classList.remove('hidden');
      showError('Error processing PSD: ' + err);
    }
  }

  // --- Create Layer Card ---
  function createLayerCard(layer, index) {
    const card = document.createElement('div');
    card.className = 'layer-card';
    card.style.animationDelay = `${index * 80}ms`;
    const ext = layer.name.split('.').pop().toLowerCase();
    const isFullSize = layer.name.startsWith('_');

    const preview = document.createElement('div');
    preview.className = 'layer-preview';
    const img = document.createElement('img');
    img.src = layer.preview_data_url;
    img.alt = layer.name;
    preview.appendChild(img);

    const meta = document.createElement('div');
    meta.className = 'layer-meta';

    const nameDiv = document.createElement('div');
    nameDiv.className = 'layer-name';
    nameDiv.title = layer.name;
    nameDiv.textContent = layer.name;

    const dimsDiv = document.createElement('div');
    dimsDiv.className = 'layer-dims';
    dimsDiv.textContent = `${layer.width} x ${layer.height} `;

    const badge = document.createElement('span');
    badge.className = `layer-badge badge-${ext}`;
    badge.textContent = ext;
    dimsDiv.appendChild(badge);

    if (isFullSize) {
      const fullBadge = document.createElement('span');
      fullBadge.className = 'layer-badge badge-full';
      fullBadge.textContent = 'full';
      dimsDiv.appendChild(fullBadge);
    }

    meta.appendChild(nameDiv);
    meta.appendChild(dimsDiv);
    card.appendChild(preview);
    card.appendChild(meta);

    // Native file drag via tauri-plugin-drag
    card.draggable = false; // disable HTML5 drag, use native OS drag instead
    card.addEventListener('mousedown', async (e) => {
      if (e.button !== 0) return;
      const startX = e.clientX;
      const startY = e.clientY;
      const onMove = async (moveEvt) => {
        const dx = moveEvt.clientX - startX;
        const dy = moveEvt.clientY - startY;
        if (Math.abs(dx) + Math.abs(dy) > 5) {
          document.removeEventListener('mousemove', onMove);
          document.removeEventListener('mouseup', onUp);
          try {
            const { Channel } = window.__TAURI__.core;
            const ch = new Channel();
            await invoke('plugin:drag|start_drag', {
              item: [layer.saved_path],
              image: layer.saved_path,
              options: {},
              onEvent: ch,
            });
          } catch (err) {
            console.error('Native drag failed:', err);
          }
        }
      };
      const onUp = () => {
        document.removeEventListener('mousemove', onMove);
        document.removeEventListener('mouseup', onUp);
      };
      document.addEventListener('mousemove', onMove);
      document.addEventListener('mouseup', onUp);
    });

    return card;
  }

  // --- Error display ---
  function showError(msg) {
    const el = document.createElement('div');
    el.style.cssText = 'position:fixed;top:20px;left:50%;transform:translateX(-50%);background:#dc2626;color:#fff;padding:12px 24px;border-radius:8px;font-size:14px;z-index:9999;animation:pop-in 0.3s ease';
    el.textContent = msg;
    document.body.appendChild(el);
    setTimeout(() => el.remove(), 5000);
  }

  // --- Actions ---
  openFolderBtn.addEventListener('click', async () => {
    if (currentOutputDir) {
      try {
        await invoke('open_folder', { path: currentOutputDir });
      } catch (e) {
        console.error('Failed to open folder:', e);
      }
    }
  });

  resetBtn.addEventListener('click', () => {
    results.classList.add('hidden');
    dropZone.classList.remove('hidden');
    layersGrid.innerHTML = '';
    currentOutputDir = '';
  });
}
