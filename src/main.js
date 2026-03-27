document.addEventListener('DOMContentLoaded', () => {
  init();
});

async function init() {
  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;

  function getOpenDialog() {
    if (window.__TAURI__?.dialog?.open) {
      return window.__TAURI__.dialog.open;
    }
    return null;
  }

  const dropZone = document.getElementById('drop-zone');
  const dropZoneEmpty = document.getElementById('drop-zone-empty');
  const dropZonePreview = document.getElementById('drop-zone-preview');
  const sourcePreviewImg = document.getElementById('source-preview-img');
  const sourceName = document.getElementById('source-name');
  const processing = document.getElementById('processing');
  const results = document.getElementById('results');
  const layersGrid = document.getElementById('layers-grid');
  const resultsTitle = document.getElementById('results-title');
  const headerActions = document.getElementById('header-actions');
  const openFolderBtn = document.getElementById('open-folder-btn');
  const resetBtn = document.getElementById('reset-btn');

  let currentOutputDir = '';
  let currentFilePath = '';

  // --- Drop Zone Click ---
  dropZone.addEventListener('click', async () => {
    const openDialog = getOpenDialog();
    if (openDialog) {
      try {
        const selected = await openDialog({
          multiple: false,
          filters: [{ name: 'Photoshop Files', extensions: ['psd'] }],
        });
        if (selected) {
          processFile(typeof selected === 'string' ? selected : selected.path);
        }
      } catch (err) {
        console.error('Dialog error:', err);
        showError('Failed to open file dialog: ' + err);
      }
    } else {
      console.error('Dialog API not available.');
      showError('File dialog not available — check app permissions.');
    }
  });

  // --- Tauri v2 file drop events ---
  listen('tauri://drag-drop', (event) => {
    dropZone.classList.remove('drag-over');
    const paths = event.payload.paths;
    if (paths && paths.length > 0) {
      const file = paths[0];
      if (file.toLowerCase().endsWith('.psd')) {
        processFile(file);
      } else {
        showError('Please drop a .psd file');
      }
    }
  });

  listen('tauri://drag-enter', () => {
    dropZone.classList.add('drag-over');
  });

  listen('tauri://drag-leave', () => {
    dropZone.classList.remove('drag-over');
  });

  // --- File change watcher ---
  listen('file-changed', () => {
    if (currentFilePath) {
      processFile(currentFilePath);
    }
  });

  // --- Process File ---
  async function processFile(filePath) {
    // Show spinner, hide layers while processing
    results.classList.add('hidden');
    processing.classList.remove('hidden');
    layersGrid.innerHTML = '';

    try {
      const result = await invoke('process_psd', { filePath });
      currentOutputDir = result.output_dir;
      currentFilePath = filePath;

      // Show the composite preview in the drop zone (image stays visible)
      dropZone.classList.add('has-file');
      dropZoneEmpty.classList.add('hidden');
      dropZonePreview.classList.remove('hidden');
      sourcePreviewImg.src = result.composite_preview;
      sourceName.textContent = result.file_name + '.psd';

      // Show header buttons
      headerActions.classList.remove('hidden');

      // Start watching for file changes
      try {
        await invoke('watch_file', { filePath });
      } catch (e) {
        console.warn('File watch failed:', e);
      }

      processing.classList.add('hidden');
      results.classList.remove('hidden');

      const count = result.layers.length;
      resultsTitle.textContent = `${count} layer${count !== 1 ? 's' : ''} extracted`;

      result.layers.forEach((layer, index) => {
        const card = createLayerCard(layer, index);
        layersGrid.appendChild(card);
      });
    } catch (err) {
      processing.classList.add('hidden');
      if (!currentFilePath) {
        dropZone.classList.remove('has-file');
        dropZoneEmpty.classList.remove('hidden');
        dropZonePreview.classList.add('hidden');
      } else {
        results.classList.remove('hidden');
      }
      showError('Error processing PSD: ' + err);
    }
  }

  // --- Create Layer Card ---
  function createLayerCard(layer, index) {
    const card = document.createElement('div');
    card.className = 'layer-card';
    card.style.animationDelay = `${index * 80}ms`;
    card.draggable = true;

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

    // Native file drag
    card.addEventListener('dragstart', (e) => {
      e.dataTransfer.setData('text/uri-list', 'file://' + layer.saved_path);
      e.dataTransfer.setData('text/plain', layer.saved_path);
      e.dataTransfer.effectAllowed = 'copy';
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

  resetBtn.addEventListener('click', async () => {
    results.classList.add('hidden');
    headerActions.classList.add('hidden');
    dropZone.classList.remove('has-file');
    dropZoneEmpty.classList.remove('hidden');
    dropZonePreview.classList.add('hidden');
    layersGrid.innerHTML = '';
    currentOutputDir = '';
    currentFilePath = '';
    try {
      await invoke('unwatch_file');
    } catch (e) {
      // ignore
    }
  });
}
