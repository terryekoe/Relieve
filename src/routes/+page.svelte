<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { 
    Mic, MicOff, Monitor, CheckCircle2, 
    X, Search, Layers, Clock,
    Eye, EyeOff, Send, Sparkles, Play,
    DownloadCloud, Settings, Palette, Music,
    BookOpen, Check, ChevronRight, Sliders, Volume2,
    Plus, Trash2, Edit2, Type, ArrowLeft,
    Sun, Moon, Image, Upload
  } from 'lucide-svelte';

  interface ScriptureVerse {
    book: String;
    chapter: number;
    verse: number;
    text: string;
  }

  interface ScripturePassage {
    reference: string;
    translation: string;
    book: string;
    chapter: number;
    verse_start: number;
    verse_end?: number;
    verses: ScriptureVerse[];
    combined_text: string;
  }

  interface AudioDeviceInfo {
    name: string;
    is_default: boolean;
  }

  interface SongStanza {
    label: string;
    text: string;
  }

  interface SongItem {
    id: string;
    title: string;
    author: string;
    category: string;
    stanzas: SongStanza[];
  }

  // Reactive state using Svelte 5 Runes
  let audioActive = $state(false);
  let audioLevel = $state(0);
  let devices = $state<AudioDeviceInfo[]>([]);
  let selectedDevice = $state<string>('');
  let autoProject = $state(false);
  let activeTranslation = $state('KJV');
  let projectorOpen = $state(false);
  let projectorBlackout = $state(false);
  let projectorClear = $state(false);
  let projectorTheme = $state('dark');
  let projectorFontFamily = $state<string>('serif');
  let projectorFontSize = $state<string>('auto');

  // FreeShow-style Scripture Layout & Presentation settings
  const STORAGE_KEY_REF_POS = 'relieve_ref_position';
  const STORAGE_KEY_COVERAGE = 'relieve_verse_coverage';
  const STORAGE_KEY_AUTO_FIT = 'relieve_auto_fit';
  const STORAGE_KEY_AUTO_GROW = 'relieve_auto_grow';
  const STORAGE_KEY_AUTO_SHRINK = 'relieve_auto_shrink';
  const STORAGE_KEY_MIN_FONT_SIZE = 'relieve_min_font_size';
  const STORAGE_KEY_MAX_FONT_SIZE = 'relieve_max_font_size';

  let projectorRefPosition = $state<string>('bottom-center');
  let projectorVerseCoverage = $state<number>(85);
  let projectorAutoFit = $state<boolean>(true);
  let projectorAutoGrow = $state<boolean>(true);
  let projectorAutoShrink = $state<boolean>(true);
  let projectorMinFontSize = $state<number>(28);
  let projectorMaxFontSize = $state<number>(84);

  // Monitor preview canvas measurement for 1:1 proportional scaling
  let monitorContainerWidth = $state(380);

  // System Fonts Discovery & Search
  let systemFonts = $state<string[]>([
    'Arial', 'Baskerville', 'Calibri', 'Cambria', 'Charter', 'Courier New',
    'Didot', 'Futura', 'Georgia', 'Gill Sans', 'Helvetica', 'Helvetica Neue',
    'Menlo', 'Palatino', 'SF Pro Display', 'Times New Roman', 'Trebuchet MS', 'Verdana'
  ]);
  let fontSearchQuery = $state('');

  // IDE-style Dock & Studio Navigation (VS Code approach)
  let activeDockView = $state<'scripture' | 'songs' | 'media' | 'settings'>('scripture');
  let songStudioView = $state<'library' | 'editor'>('library');
  let showThemePopover = $state(false);
  // Application Interface Theme State (Dark, Light, System)
  const STORAGE_KEY_APP_THEME = 'relieve_ui_theme';
  type AppTheme = 'dark' | 'light' | 'system';
  let appTheme = $state<AppTheme>('dark');
  let systemPrefersDark = $state(true);
  let effectiveTheme = $derived.by<'dark' | 'light'>(() => {
    if (appTheme === 'system') {
      return systemPrefersDark ? 'dark' : 'light';
    }
    return appTheme;
  });

  function setAppTheme(t: AppTheme) {
    appTheme = t;
    try {
      localStorage.setItem(STORAGE_KEY_APP_THEME, t);
    } catch (e) {
      console.warn('Failed to save UI theme:', e);
    }
  }

  function toggleAppTheme() {
    if (effectiveTheme === 'light') {
      setAppTheme('dark');
    } else {
      setAppTheme('light');
    }
  }

  // Font family resolution (CSS style string)
  let projectorFontFamilyStyle = $derived.by(() => {
    if (!projectorFontFamily || projectorFontFamily === 'serif') return 'Georgia, Cambria, "Times New Roman", serif';
    if (projectorFontFamily === 'sans') return 'system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
    if (projectorFontFamily === 'slab') return '"SF Mono", Menlo, Courier, monospace';
    return `"${projectorFontFamily}", system-ui, -apple-system, sans-serif`;
  });

  // FreeShow-style numeric check
  let isNumericFontSize = $derived.by(() => {
    if (!projectorFontSize || projectorFontSize === 'auto') return false;
    const n = Number(projectorFontSize);
    return !isNaN(n) && n > 0;
  });

  // FreeShow-style dynamic font sizing engine
  function computePreviewFontSize(
    textLen: number,
    baseSizeStr: string,
    isAutoFit: boolean,
    canGrow: boolean,
    canShrink: boolean,
    minS: number,
    maxS: number
  ): number {
    const num = Number(baseSizeStr);
    const base = (!isNaN(num) && num > 0) ? num : 48;

    if (!isAutoFit && baseSizeStr !== 'auto') {
      return Math.max(16, Math.min(160, base));
    }

    let target: number;
    if (textLen <= 45) {
      target = 82;
    } else if (textLen <= 90) {
      target = 70;
    } else if (textLen <= 160) {
      target = 56;
    } else if (textLen <= 240) {
      target = 46;
    } else if (textLen <= 360) {
      target = 38;
    } else if (textLen <= 500) {
      target = 32;
    } else {
      target = 26;
    }

    if (base !== 48) {
      target = Math.round(target * (base / 48));
    }

    if (!canGrow && target > base) {
      target = base;
    }

    if (!canShrink && target < base) {
      target = base;
    }

    return Math.max(minS, Math.min(maxS, target));
  }

  let dynamicFontSizePx = $derived.by(() => {
    const len = currentPassage ? currentPassage.combined_text.length : 120;
    return computePreviewFontSize(
      len,
      projectorFontSize,
      projectorAutoFit,
      projectorAutoGrow,
      projectorAutoShrink,
      projectorMinFontSize,
      projectorMaxFontSize
    );
  });

  // Dynamic Theme Background Styles (Atmospheric Worship Backdrops)
  let projectorThemeBackgroundStyle = $derived.by(() => {
    if (projectorBlackout) return 'background: #000000; color: #ffffff;';
    if (projectorTheme === 'light') return 'background: #f8fafc; color: #0f172a;';
    if (projectorTheme === 'lower-third') return 'background: transparent; color: #ffffff;';
    if (projectorTheme === 'minimal-black' || projectorTheme === 'black') return 'background: #000000; color: #ffffff;';
    if (projectorTheme === 'gold') {
      return 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #1c1a2e 0%, #0c0b16 55%, #05040a 100%); color: #fef08a;';
    }
    if (projectorTheme === 'celestial') {
      return 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #351c54 0%, #1a0f30 55%, #0a0514 100%); color: #ffffff;';
    }
    if (projectorTheme === 'emerald') {
      return 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #063826 0%, #031c13 55%, #010a07 100%); color: #a7f3d0;';
    }
    if (projectorTheme === 'crimson') {
      return 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #420d18 0%, #20060c 55%, #0d0205 100%); color: #fecdd3;';
    }
    if (projectorTheme === 'ocean') {
      return 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #0a3a40 0%, #051d20 55%, #020c0e 100%); color: #a5f3fc;';
    }
    if (projectorTheme === 'sunset') {
      return 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #3d1c24 0%, #1d0e14 55%, #0b0508 100%); color: #fed7aa;';
    }
    if (projectorTheme.startsWith('url(') || projectorTheme.startsWith('data:') || projectorTheme.startsWith('http')) {
      const url = projectorTheme.startsWith('url(') ? projectorTheme : `url("${projectorTheme}")`;
      return `background: ${url} center/cover no-repeat, #000000; color: #ffffff;`;
    }
    // Default 'dark' / 'atmospheric': Gorgeous worship ambient celestial blue gradient as seen in the screenshots!
    return 'background: radial-gradient(ellipse 95% 75% at 50% 35%, #183d63 0%, #0d223c 55%, #050e1a 100%); color: #ffffff;';
  });

  // Filtered system fonts for picker
  let filteredSystemFonts = $derived.by(() => {
    const q = fontSearchQuery.trim().toLowerCase();
    if (!q) return systemFonts.slice(0, 150);
    return systemFonts.filter(f => f.toLowerCase().includes(q)).slice(0, 150);
  });

  // Songs & Worship State
  const STORAGE_KEY_CUSTOM_SONGS = 'relieve_custom_songs';
  let customSongs = $state<SongItem[]>([]);
  let songSearch = $state('');
  let selectedSongId = $state('amazing-grace');
  let activeSongStanzaKey = $state<string | null>(null);

  // Song Creator Form State
  let editingSongId = $state<string | null>(null);
  let newSongTitle = $state('');
  let newSongAuthor = $state('');
  let newSongCategory = $state('Worship');
  let newSongRawLyrics = $state('');
  let parsedStanzas = $state<SongStanza[]>([]);

  const worshipSongs: SongItem[] = [
    {
      id: 'amazing-grace',
      title: 'Amazing Grace',
      author: 'John Newton',
      category: 'Hymn',
      stanzas: [
        {
          label: 'Verse 1',
          text: 'Amazing grace! how sweet the sound,\nThat saved a wretch like me!\nI once was lost, but now am found,\nWas blind, but now I see.'
        },
        {
          label: 'Verse 2',
          text: '’Twas grace that taught my heart to fear,\nAnd grace my fears relieved;\nHow precious did that grace appear\nThe hour I first believed!'
        },
        {
          label: 'Verse 3',
          text: 'Through many dangers, toils and snares,\nI have already come;\n’Tis grace hath brought me safe thus far,\nAnd grace will lead me home.'
        },
        {
          label: 'Verse 4',
          text: 'When we’ve been there ten thousand years,\nBright shining as the sun,\nWe’ve no less days to sing God’s praise\nThan when we first begun.'
        }
      ]
    },
    {
      id: 'how-great-thou-art',
      title: 'How Great Thou Art',
      author: 'Stuart K. Hine',
      category: 'Hymn',
      stanzas: [
        {
          label: 'Verse 1',
          text: 'O Lord my God, when I in awesome wonder,\nConsider all the worlds Thy Hands have made;\nI see the stars, I hear the rolling thunder,\nThy power throughout the universe displayed.'
        },
        {
          label: 'Chorus',
          text: 'Then sings my soul, My Saviour God, to Thee,\nHow great Thou art, How great Thou art.\nThen sings my soul, My Saviour God, to Thee,\nHow great Thou art, How great Thou art!'
        },
        {
          label: 'Verse 2',
          text: 'When through the woods and forest glades I wander,\nAnd hear the birds sing sweetly in the trees;\nWhen I look down from lofty mountain grandeur,\nAnd see the brook and feel the gentle breeze.'
        },
        {
          label: 'Verse 3',
          text: 'And when I think that God, His Son not sparing,\nSent Him to die, I scarce can take it in;\nThat on the Cross, my burden gladly bearing,\nHe bled and died to take away my sin.'
        }
      ]
    },
    {
      id: 'great-is-thy-faithfulness',
      title: 'Great Is Thy Faithfulness',
      author: 'Thomas Chisholm',
      category: 'Hymn',
      stanzas: [
        {
          label: 'Verse 1',
          text: 'Great is Thy faithfulness, O God my Father,\nThere is no shadow of turning with Thee;\nThou changest not, Thy compassions, they fail not;\nAs Thou hast been Thou forever wilt be.'
        },
        {
          label: 'Chorus',
          text: 'Great is Thy faithfulness! Great is Thy faithfulness!\nMorning by morning new mercies I see;\nAll I have needed Thy hand hath provided—\nGreat is Thy faithfulness, Lord, unto me!'
        },
        {
          label: 'Verse 2',
          text: 'Summer and winter, and springtime and harvest,\nSun, moon and stars in their courses above,\nJoin with all nature in manifold witness\nTo Thy great faithfulness, mercy and love.'
        },
        {
          label: 'Verse 3',
          text: 'Pardon for sin and a peace that endureth,\nThine own dear presence to cheer and to guide;\nStrength for today and bright hope for tomorrow,\nBlessings all mine, with ten thousand beside!'
        }
      ]
    },
    {
      id: 'way-maker',
      title: 'Way Maker',
      author: 'Sinach',
      category: 'Contemporary',
      stanzas: [
        {
          label: 'Verse 1',
          text: 'You are here, moving in our midst;\nI worship You, I worship You.\nYou are here, working in this place;\nI worship You, I worship You.'
        },
        {
          label: 'Chorus',
          text: 'You are Way Maker, Miracle Worker,\nPromise Keeper, Light in the darkness,\nMy God, that is who You are!'
        },
        {
          label: 'Verse 2',
          text: 'You are here, touching every heart;\nI worship You, I worship You.\nYou are here, healing every heart;\nI worship You, I worship You.'
        },
        {
          label: 'Bridge',
          text: "Even when I don't see it, You're working.\nEven when I don't feel it, You're working.\nYou never stop, You never stop working."
        }
      ]
    },
    {
      id: '10000-reasons',
      title: '10,000 Reasons (Bless the Lord)',
      author: 'Matt Redman',
      category: 'Praise',
      stanzas: [
        {
          label: 'Chorus',
          text: 'Bless the Lord, O my soul, O my soul;\nWorship His holy name.\nSing like never before, O my soul;\nI’ll worship Your holy name.'
        },
        {
          label: 'Verse 1',
          text: 'The sun comes up, it’s a new day dawning;\nIt’s time to sing Your song again.\nWhatever may pass, and whatever lies before me,\nLet me be singing when the evening comes.'
        },
        {
          label: 'Verse 2',
          text: 'You’re rich in love, and You’re slow to anger.\nYour name is great, and Your heart is kind.\nFor all Your goodness, I will keep on singing:\nTen thousand reasons for my heart to find.'
        }
      ]
    },
    {
      id: 'in-christ-alone',
      title: 'In Christ Alone',
      author: 'Keith Getty, Stuart Townend',
      category: 'Hymn',
      stanzas: [
        {
          label: 'Verse 1',
          text: 'In Christ alone my hope is found;\nHe is my light, my strength, my song;\nThis cornerstone, this solid ground,\nFirm through the fiercest drought and storm.'
        },
        {
          label: 'Verse 2',
          text: 'In Christ alone, who took on flesh,\nFullness of God in helpless babe!\nThis gift of love and righteousness,\nScorned by the ones He came to save.'
        },
        {
          label: 'Verse 3',
          text: 'There in the ground His body lay,\nLight of the world by darkness slain;\nThen bursting forth in glorious day,\nUp from the grave He rose again!'
        }
      ]
    }
  ];

  let allSongs = $derived([...customSongs, ...worshipSongs]);

  let filteredSongs = $derived(
    allSongs.filter(s => 
      s.title.toLowerCase().includes(songSearch.toLowerCase()) ||
      s.author.toLowerCase().includes(songSearch.toLowerCase()) ||
      s.stanzas.some(st => st.text.toLowerCase().includes(songSearch.toLowerCase()))
    )
  );

  let selectedSong = $derived(
    allSongs.find(s => s.id === selectedSongId) || allSongs[0] || worshipSongs[0]
  );

  // Offline AI Model Engine State (Base.en recommended for African/regional accents)
  let modelReady = $state(false);
  let modelDownloading = $state(false);
  let downloadPercent = $state(0);
  let modelName = $state('ggml-base.en.bin');
  let modelPath = $state<string | null>(null);

  // Transcripts and detection
  let liveTranscript = $state('');
  let transcriptHistory = $state<string[]>([]);
  let detectedPassage = $state<ScripturePassage | null>(null);
  let countdown = $state(3.0);
  let countdownTimer: any = null;

  // Active Presentation
  let currentPassage = $state<ScripturePassage | null>(null);

  // Manual Search
  let searchQuery = $state('');
  let searchResults = $state<ScriptureVerse[]>([]);
  let isSearching = $state(false);

  let unlisteners: UnlistenFn[] = [];
  let mediaQueryList: MediaQueryList | null = null;
  let mediaQueryListener: ((e: MediaQueryListEvent) => void) | null = null;

  onMount(async () => {
    try {
      const savedTheme = localStorage.getItem(STORAGE_KEY_APP_THEME) as AppTheme | null;
      if (savedTheme && (savedTheme === 'dark' || savedTheme === 'light' || savedTheme === 'system')) {
        appTheme = savedTheme;
      }
    } catch (e) {
      console.warn('Failed to load UI theme:', e);
    }

    if (typeof window !== 'undefined' && window.matchMedia) {
      mediaQueryList = window.matchMedia('(prefers-color-scheme: dark)');
      systemPrefersDark = mediaQueryList.matches;
      mediaQueryListener = (e: MediaQueryListEvent) => {
        systemPrefersDark = e.matches;
      };
      mediaQueryList.addEventListener('change', mediaQueryListener);
    }

    try {
      const saved = localStorage.getItem(STORAGE_KEY_CUSTOM_SONGS);
      if (saved) {
        customSongs = JSON.parse(saved);
      }
      const savedMedia = localStorage.getItem(STORAGE_KEY_CUSTOM_MEDIA);
      if (savedMedia) {
        customMedia = JSON.parse(savedMedia);
      }
    } catch (e) {
      console.warn('Failed to load custom songs or media:', e);
    }

    await refreshAudioDevices();
    await fetchStatus();
    await checkModelStatus();

    try {
      const sf: string[] = await invoke('get_system_fonts');
      if (sf && sf.length > 0) {
        systemFonts = sf;
      }
    } catch (e) {
      console.warn('get_system_fonts error:', e);
    }

    // Listen for events from Rust
    try {
      const u1 = await listen<number>('audio-level', (event) => {
        audioLevel = event.payload;
      });
      unlisteners.push(u1);

      const u2 = await listen<string>('live-transcript', (event) => {
        liveTranscript = event.payload;
        transcriptHistory = [event.payload, ...transcriptHistory.slice(0, 19)];
      });
      unlisteners.push(u2);

      const u3 = await listen<ScripturePassage>('scripture-detected', (event) => {
        handleScriptureDetected(event.payload);
      });
      unlisteners.push(u3);

      const u4 = await listen<ScripturePassage>('present-slide', (event) => {
        currentPassage = event.payload;
        projectorClear = false;
        projectorBlackout = false;
      });
      unlisteners.push(u4);

      const u5 = await listen('projector-clear', () => {
        projectorClear = true;
      });
      unlisteners.push(u5);

      const u6 = await listen<boolean>('projector-blackout', (event) => {
        projectorBlackout = event.payload;
      });
      unlisteners.push(u6);

      const u7 = await listen<{ model: string; percent: number; downloaded: number; total: number; complete?: boolean }>(
        'model-download-progress',
        (event) => {
          modelDownloading = true;
          downloadPercent = event.payload.percent;
          if (event.payload.complete || event.payload.percent >= 100) {
            modelReady = true;
            modelDownloading = false;
          }
        }
      );
      unlisteners.push(u7);

      const u8 = await listen<any>(
        'projector-typography',
        (event) => {
          const ff = event.payload.fontFamily || event.payload.font_family;
          const fs = event.payload.fontSize || event.payload.font_size;
          if (ff) projectorFontFamily = String(ff);
          if (fs) projectorFontSize = String(fs);
        }
      );
      unlisteners.push(u8);

      const u9 = await listen<any>(
        'projector-layout',
        (event) => {
          const p = event.payload;
          if (p.refPosition !== undefined) projectorRefPosition = p.refPosition;
          if (p.verseCoverage !== undefined) projectorVerseCoverage = Number(p.verseCoverage);
          if (p.autoFit !== undefined) projectorAutoFit = Boolean(p.autoFit);
          if (p.autoGrow !== undefined) projectorAutoGrow = Boolean(p.autoGrow);
          if (p.autoShrink !== undefined) projectorAutoShrink = Boolean(p.autoShrink);
          if (p.minFontSize !== undefined) projectorMinFontSize = Number(p.minFontSize);
          if (p.maxFontSize !== undefined) projectorMaxFontSize = Number(p.maxFontSize);
        }
      );
      unlisteners.push(u9);
    } catch (e) {
      console.warn('Tauri event listener initialization:', e);
    }

    // Keyboard shortcuts: Space to push, Esc to dismiss, B for blackout, C for clear
    window.addEventListener('keydown', handleKeyDown);
  });

  onDestroy(() => {
    unlisteners.forEach((u) => u());
    if (countdownTimer) clearInterval(countdownTimer);
    if (mediaQueryList && mediaQueryListener) {
      mediaQueryList.removeEventListener('change', mediaQueryListener);
    }
    window.removeEventListener('keydown', handleKeyDown);
  });

  function handleKeyDown(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
      return;
    }
    if (e.key === 'Escape') {
      if (songStudioView === 'editor') {
        songStudioView = 'library';
        return;
      }
      if (showUploadModal) {
        showUploadModal = false;
        return;
      }
      if (activeDockView === 'settings' || activeDockView === 'media') {
        activeDockView = 'scripture';
        return;
      }
      if (showThemePopover) {
        showThemePopover = false;
        return;
      }
      if (detectedPassage) {
        e.preventDefault();
        dismissDetected();
        return;
      }
    }
    if (e.code === 'Space' && detectedPassage) {
      e.preventDefault();
      confirmPresent();
    } else if (e.key.toLowerCase() === 'b') {
      toggleBlackout();
    } else if (e.key.toLowerCase() === 'c') {
      clearSlide();
    }
  }

  async function fetchStatus() {
    try {
      const status: any = await invoke('get_status');
      audioActive = status.audio_active;
      autoProject = status.auto_project;
      activeTranslation = status.active_translation;
      projectorBlackout = status.projector_blackout;
      projectorClear = status.projector_clear;
      projectorTheme = status.projector_theme || 'dark';
      projectorFontFamily = (status.projector_font_family as any) || 'serif';
      projectorFontSize = (status.projector_font_size as any) || 'auto';
      currentPassage = status.current_passage;

      const savedRefPos = localStorage.getItem(STORAGE_KEY_REF_POS);
      projectorRefPosition = savedRefPos || status.projector_ref_position || 'bottom-center';

      const savedCoverage = localStorage.getItem(STORAGE_KEY_COVERAGE);
      projectorVerseCoverage = savedCoverage ? Number(savedCoverage) : (Number(status.projector_verse_coverage) || 85);

      const savedAutoFit = localStorage.getItem(STORAGE_KEY_AUTO_FIT);
      projectorAutoFit = savedAutoFit !== null ? savedAutoFit === 'true' : (status.projector_auto_fit ?? true);

      const savedAutoGrow = localStorage.getItem(STORAGE_KEY_AUTO_GROW);
      projectorAutoGrow = savedAutoGrow !== null ? savedAutoGrow === 'true' : (status.projector_auto_grow ?? true);

      const savedAutoShrink = localStorage.getItem(STORAGE_KEY_AUTO_SHRINK);
      projectorAutoShrink = savedAutoShrink !== null ? savedAutoShrink === 'true' : (status.projector_auto_shrink ?? true);

      const savedMinFont = localStorage.getItem(STORAGE_KEY_MIN_FONT_SIZE);
      projectorMinFontSize = savedMinFont ? Number(savedMinFont) : (Number(status.projector_min_font_size) || 28);

      const savedMaxFont = localStorage.getItem(STORAGE_KEY_MAX_FONT_SIZE);
      projectorMaxFontSize = savedMaxFont ? Number(savedMaxFont) : (Number(status.projector_max_font_size) || 84);
    } catch (e) {
      console.warn('fetchStatus error:', e);
    }
  }

  async function checkModelStatus() {
    try {
      const status: any = await invoke('get_model_status');
      modelReady = status.is_ready;
      modelName = status.model_name || 'ggml-base.en.bin';
      modelPath = status.path;
      if (!modelReady && !modelDownloading) {
        // Auto-download offline model on first launch
        startModelDownload();
      }
    } catch (e) {
      console.warn('get_model_status error:', e);
    }
  }

  async function startModelDownload() {
    if (modelDownloading) return;
    modelDownloading = true;
    downloadPercent = 0;
    try {
      const path: string = await invoke('download_model', { modelName: 'ggml-base.en.bin' });
      modelPath = path;
      modelReady = true;
    } catch (e) {
      console.error('download_model error:', e);
    } finally {
      modelDownloading = false;
    }
  }

  async function refreshAudioDevices() {
    try {
      devices = await invoke('get_audio_devices');
      const def = devices.find((d) => d.is_default);
      if (def && !selectedDevice) {
        selectedDevice = def.name;
      }
    } catch (e) {
      console.warn('get_audio_devices error:', e);
    }
  }

  async function toggleAudio() {
    try {
      if (audioActive) {
        await invoke('stop_audio_capture');
        audioActive = false;
        audioLevel = 0;
      } else {
        await invoke('start_audio_capture', {
          deviceName: selectedDevice || null,
        });
        audioActive = true;
      }
    } catch (e) {
      console.error('Audio toggle error:', e);
    }
  }

  function handleScriptureDetected(passage: ScripturePassage) {
    detectedPassage = passage;
    if (countdownTimer) clearInterval(countdownTimer);

    if (autoProject) {
      // Auto-pilot: immediately presented by Rust
      confirmPresent();
    } else {
      // Co-Pilot: 3-second countdown
      countdown = 3.0;
      countdownTimer = setInterval(() => {
        countdown = Math.max(0, +(countdown - 0.1).toFixed(1));
        if (countdown <= 0) {
          clearInterval(countdownTimer);
          confirmPresent();
        }
      }, 100);
    }
  }

  async function confirmPresent() {
    if (!detectedPassage) return;
    const passageToPresent = detectedPassage;
    if (countdownTimer) clearInterval(countdownTimer);
    detectedPassage = null;
    try {
      await invoke('present_passage', { passage: passageToPresent });
      currentPassage = passageToPresent;
    } catch (e) {
      console.error('present_passage error:', e);
    }
  }

  function dismissDetected() {
    if (countdownTimer) clearInterval(countdownTimer);
    detectedPassage = null;
  }

  async function toggleAutoProject() {
    autoProject = !autoProject;
    await invoke('set_auto_project', { enabled: autoProject });
  }

  async function changeTranslation(trans: string) {
    activeTranslation = trans;
    await invoke('set_translation', { translation: trans });
    if (currentPassage) {
      // Reload current passage in new translation
      try {
        const reloaded: ScripturePassage = await invoke('lookup_scripture', {
          book: currentPassage.book,
          chapter: currentPassage.chapter,
          verseStart: currentPassage.verse_start,
          verseEnd: currentPassage.verse_end || null,
          translation: trans,
        });
        await invoke('present_passage', { passage: reloaded });
      } catch (e) {
        console.warn('Failed to reload passage in translation:', e);
      }
    }
  }

  async function toggleProjector() {
    try {
      projectorOpen = await invoke('toggle_projector_window');
    } catch (e) {
      console.error('toggle_projector_window error:', e);
    }
  }

  async function clearSlide() {
    try {
      await invoke('clear_presentation');
      projectorClear = true;
    } catch (e) {
      console.error('clear_presentation error:', e);
    }
  }

  async function toggleBlackout() {
    try {
      projectorBlackout = !projectorBlackout;
      await invoke('blackout_presentation', { blackout: projectorBlackout });
    } catch (e) {
      console.error('blackout_presentation error:', e);
    }
  }

  async function setTheme(theme: string) {
    projectorTheme = theme;
    await invoke('set_projector_theme', { theme });
  }

  async function selectQuickVerse(ref: string) {
    try {
      await invoke('process_transcript', { transcript: ref });
    } catch (e) {
      console.error('Quick verse selection error:', e);
    }
  }

  async function handleSearch() {
    if (!searchQuery.trim()) return;
    isSearching = true;
    try {
      // First check if user typed a reference directly e.g. "John 3:16"
      const detected = await invoke<ScripturePassage | null>('process_transcript', {
        transcript: searchQuery,
      });
      if (detected) {
        searchResults = detected.verses;
      } else {
        searchResults = await invoke('search_scripture', {
          query: searchQuery,
          translation: activeTranslation,
        });
      }
    } catch (e) {
      console.error('search error:', e);
    } finally {
      isSearching = false;
    }
  }

  async function presentSearchedVerse(v: ScriptureVerse) {
    try {
      const passage: ScripturePassage = await invoke('lookup_scripture', {
        book: v.book,
        chapter: v.chapter,
        verseStart: v.verse,
        verseEnd: null,
        translation: activeTranslation,
      });
      await invoke('present_passage', { passage });
    } catch (e) {
      console.error('presentSearchedVerse error:', e);
    }
  }

  async function presentSongStanza(song: SongItem, stanza: SongStanza, index: number) {
    activeSongStanzaKey = `${song.id}-${stanza.label}`;
    const passage: ScripturePassage = {
      reference: `${song.title} (${stanza.label})`,
      translation: 'Lyrics',
      book: song.title,
      chapter: 1,
      verse_start: index + 1,
      verse_end: undefined,
      verses: [],
      combined_text: stanza.text
    };
    try {
      await invoke('present_passage', { passage });
      currentPassage = passage;
      projectorClear = false;
      projectorBlackout = false;
    } catch (e) {
      console.error('presentSongStanza error:', e);
    }
  }

  async function setTypography(family: string, size: string) {
    projectorFontFamily = family;
    projectorFontSize = size;
    try {
      await invoke('set_projector_typography', {
        fontFamily: family,
        fontSize: size
      });
    } catch (e) {
      console.warn('Failed to set projector typography:', e);
    }
  }

  function adjustFontSize(delta: number) {
    let currentNum = isNumericFontSize ? Number(projectorFontSize) : 48;
    let nextNum = Math.max(16, Math.min(160, currentNum + delta));
    setTypography(projectorFontFamily, String(nextNum));
  }

  async function setLayout(updates: {
    refPosition?: string;
    verseCoverage?: number;
    autoFit?: boolean;
    autoGrow?: boolean;
    autoShrink?: boolean;
    minFontSize?: number;
    maxFontSize?: number;
  }) {
    if (updates.refPosition !== undefined) {
      projectorRefPosition = updates.refPosition;
      localStorage.setItem(STORAGE_KEY_REF_POS, updates.refPosition);
    }
    if (updates.verseCoverage !== undefined) {
      projectorVerseCoverage = updates.verseCoverage;
      localStorage.setItem(STORAGE_KEY_COVERAGE, String(updates.verseCoverage));
    }
    if (updates.autoFit !== undefined) {
      projectorAutoFit = updates.autoFit;
      localStorage.setItem(STORAGE_KEY_AUTO_FIT, String(updates.autoFit));
    }
    if (updates.autoGrow !== undefined) {
      projectorAutoGrow = updates.autoGrow;
      localStorage.setItem(STORAGE_KEY_AUTO_GROW, String(updates.autoGrow));
    }
    if (updates.autoShrink !== undefined) {
      projectorAutoShrink = updates.autoShrink;
      localStorage.setItem(STORAGE_KEY_AUTO_SHRINK, String(updates.autoShrink));
    }
    if (updates.minFontSize !== undefined) {
      projectorMinFontSize = updates.minFontSize;
      localStorage.setItem(STORAGE_KEY_MIN_FONT_SIZE, String(updates.minFontSize));
    }
    if (updates.maxFontSize !== undefined) {
      projectorMaxFontSize = updates.maxFontSize;
      localStorage.setItem(STORAGE_KEY_MAX_FONT_SIZE, String(updates.maxFontSize));
    }

    try {
      await invoke('set_projector_layout', {
        refPosition: projectorRefPosition,
        verseCoverage: projectorVerseCoverage,
        autoFit: projectorAutoFit,
        autoGrow: projectorAutoGrow,
        autoShrink: projectorAutoShrink,
        minFontSize: projectorMinFontSize,
        maxFontSize: projectorMaxFontSize
      });
    } catch (e) {
      console.warn('Failed to set projector layout:', e);
    }
  }

  function toggleAutoFit() {
    const nextFit = !projectorAutoFit;
    setLayout({ autoFit: nextFit });
    if (!nextFit && projectorFontSize === 'auto') {
      setTypography(projectorFontFamily, '48');
    } else if (nextFit) {
      setTypography(projectorFontFamily, 'auto');
    }
  }

  function parseRawLyrics(raw: string) {
    if (!raw.trim()) {
      parsedStanzas = [];
      return;
    }
    const lines = raw.split(/\r?\n/);
    const stanzas: SongStanza[] = [];
    let currentLabel = 'Verse 1';
    let currentLines: string[] = [];
    let verseCounter = 1;

    const headerRegex = /^\s*\[?(verse\s*\d*|chorus\s*\d*|bridge\s*\d*|intro|outro|pre-chorus\s*\d*|stanza\s*\d*)\]?:?\s*$/i;

    for (const line of lines) {
      const trimmed = line.trim();
      const match = trimmed.match(headerRegex);

      if (match) {
        if (currentLines.length > 0) {
          stanzas.push({
            label: currentLabel,
            text: currentLines.join('\n').trim()
          });
          currentLines = [];
        }
        let cleanHeader = match[1];
        cleanHeader = cleanHeader.charAt(0).toUpperCase() + cleanHeader.slice(1);
        currentLabel = cleanHeader;
      } else if (trimmed === '') {
        if (currentLines.length > 0) {
          stanzas.push({
            label: currentLabel,
            text: currentLines.join('\n').trim()
          });
          currentLines = [];
          verseCounter++;
          currentLabel = `Verse ${verseCounter}`;
        }
      } else {
        currentLines.push(trimmed);
      }
    }

    if (currentLines.length > 0) {
      stanzas.push({
        label: currentLabel,
        text: currentLines.join('\n').trim()
      });
    }

    parsedStanzas = stanzas;
  }

  function handleLyricsInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    newSongRawLyrics = target.value;
    parseRawLyrics(newSongRawLyrics);
  }

  function openNewSongModal() {
    editingSongId = null;
    newSongTitle = '';
    newSongAuthor = '';
    newSongCategory = 'Worship';
    newSongRawLyrics = '';
    parsedStanzas = [];
    songStudioView = 'editor';
  }

  function openEditSongModal(song: SongItem) {
    editingSongId = song.id;
    newSongTitle = song.title;
    newSongAuthor = song.author;
    newSongCategory = song.category;
    newSongRawLyrics = song.stanzas.map(st => `[${st.label}]\n${st.text}`).join('\n\n');
    parseRawLyrics(newSongRawLyrics);
    songStudioView = 'editor';
  }

  function closeSongEditor() {
    songStudioView = 'library';
    editingSongId = null;
  }

  function saveSong() {
    if (!newSongTitle.trim() || parsedStanzas.length === 0) return;
    const songId = editingSongId || `custom-${Date.now()}`;
    const newSong: SongItem = {
      id: songId,
      title: newSongTitle.trim(),
      author: newSongAuthor.trim() || 'Church Worship',
      category: newSongCategory,
      stanzas: parsedStanzas
    };

    if (editingSongId) {
      customSongs = customSongs.map(s => s.id === editingSongId ? newSong : s);
    } else {
      customSongs = [newSong, ...customSongs];
    }

    try {
      localStorage.setItem(STORAGE_KEY_CUSTOM_SONGS, JSON.stringify(customSongs));
    } catch (e) {
      console.warn('Failed to persist custom songs:', e);
    }

    selectedSongId = songId;
    songStudioView = 'library';
    editingSongId = null;
  }

  function deleteCustomSong(songId: string) {
    customSongs = customSongs.filter(s => s.id !== songId);
    try {
      localStorage.setItem(STORAGE_KEY_CUSTOM_SONGS, JSON.stringify(customSongs));
    } catch (e) {
      console.warn('Failed to delete custom song:', e);
    }
    if (selectedSongId === songId) {
      selectedSongId = allSongs[0]?.id || worshipSongs[0].id;
    }
  }

  // Media & Backdrops Studio State
  interface MediaItem {
    id: string;
    title: string;
    category: 'motion' | 'stage' | 'custom';
    themeValue: string;
    thumbnailStyle: string;
    description: string;
    isCustom?: boolean;
    dateAdded?: number;
  }

  const STORAGE_KEY_CUSTOM_MEDIA = 'relieve_custom_media';
  let customMedia = $state<MediaItem[]>([]);
  let mediaSearchQuery = $state('');
  let mediaCategoryFilter = $state<'all' | 'motion' | 'stage' | 'custom'>('all');
  let showUploadModal = $state(false);
  let uploadMediaTitle = $state('');
  let uploadMediaUrl = $state('');
  let uploadMediaPreview = $state<string | null>(null);
  let uploadMediaError = $state<string | null>(null);

  const DEFAULT_MEDIA_ITEMS: MediaItem[] = [
    {
      id: 'atmospheric-blue',
      title: 'Atmospheric Blue',
      category: 'motion',
      themeValue: 'dark',
      thumbnailStyle: 'background: radial-gradient(ellipse 95% 75% at 50% 35%, #183d63 0%, #0d223c 55%, #050e1a 100%);',
      description: 'Signature celestial navy glow motion backdrop'
    },
    {
      id: 'celestial-violet',
      title: 'Celestial Violet',
      category: 'motion',
      themeValue: 'celestial',
      thumbnailStyle: 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #351c54 0%, #1a0f30 55%, #0a0514 100%);',
      description: 'Deep royal purple aurora worship gradient'
    },
    {
      id: 'emerald-sanctuary',
      title: 'Emerald Sanctuary',
      category: 'motion',
      themeValue: 'emerald',
      thumbnailStyle: 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #063826 0%, #031c13 55%, #010a07 100%);',
      description: 'Deep evergreen sacred worship ambient aura'
    },
    {
      id: 'crimson-majesty',
      title: 'Crimson Majesty',
      category: 'motion',
      themeValue: 'crimson',
      thumbnailStyle: 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #420d18 0%, #20060c 55%, #0d0205 100%);',
      description: 'Reverent deep ruby and wine ambient glow'
    },
    {
      id: 'oceanic-azure',
      title: 'Oceanic Azure',
      category: 'motion',
      themeValue: 'ocean',
      thumbnailStyle: 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #0a3a40 0%, #051d20 55%, #020c0e 100%);',
      description: 'Deep marine turquoise & sapphire depth'
    },
    {
      id: 'golden-sunset',
      title: 'Golden Sunset',
      category: 'motion',
      themeValue: 'sunset',
      thumbnailStyle: 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #3d1c24 0%, #1d0e14 55%, #0b0508 100%);',
      description: 'Warm twilight amber & coral dusk gradient'
    },
    {
      id: 'midnight-gold',
      title: 'Midnight Gold',
      category: 'stage',
      themeValue: 'gold',
      thumbnailStyle: 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #1c1a2e 0%, #0c0b16 55%, #05040a 100%);',
      description: 'Deep navy background paired with warm golden typography'
    },
    {
      id: 'obsidian-blackout',
      title: 'Obsidian Black',
      category: 'stage',
      themeValue: 'minimal-black',
      thumbnailStyle: 'background: #000000;',
      description: 'Ultra-pure black backdrop for high-contrast LED walls'
    },
    {
      id: 'studio-linen',
      title: 'Studio Light',
      category: 'stage',
      themeValue: 'light',
      thumbnailStyle: 'background: #f8fafc;',
      description: 'Clean bright daylight stage canvas with dark typography'
    },
    {
      id: 'lower-third-glass',
      title: 'Lower-Third Broadcast',
      category: 'stage',
      themeValue: 'lower-third',
      thumbnailStyle: 'background: linear-gradient(180deg, #18181b 0%, #09090b 100%);',
      description: 'Transparent lower-third broadcast banner overlay'
    }
  ];

  let allMediaItems = $derived.by<MediaItem[]>(() => {
    return [...customMedia, ...DEFAULT_MEDIA_ITEMS];
  });

  let filteredMediaItems = $derived.by<MediaItem[]>(() => {
    const q = mediaSearchQuery.trim().toLowerCase();
    return allMediaItems.filter(item => {
      const matchesCat = mediaCategoryFilter === 'all' || item.category === mediaCategoryFilter;
      const matchesQuery = !q || item.title.toLowerCase().includes(q) || item.description.toLowerCase().includes(q);
      return matchesCat && matchesQuery;
    });
  });

  async function projectMediaSlide(item: MediaItem, customCaption?: string) {
    await setTheme(item.themeValue);
    const passage: ScripturePassage = {
      reference: customCaption ? item.title : '',
      translation: 'Media',
      book: 'Media',
      chapter: 1,
      verse_start: 1,
      verse_end: undefined,
      verses: [],
      combined_text: customCaption || ''
    };
    try {
      await invoke('present_passage', { passage });
      currentPassage = passage;
      projectorClear = false;
      projectorBlackout = false;
    } catch (e) {
      console.error('projectMediaSlide error:', e);
    }
  }

  function handleMediaFileInput(e: Event) {
    const input = e.target as HTMLInputElement;
    if (input.files && input.files[0]) {
      const file = input.files[0];
      if (!file.type.startsWith('image/')) {
        uploadMediaError = 'Please choose an image file (PNG, JPG, WebP, GIF)';
        return;
      }
      if (file.size > 5 * 1024 * 1024) {
        uploadMediaError = 'File size exceeds 5MB limit.';
        return;
      }
      uploadMediaError = null;
      if (!uploadMediaTitle) {
        uploadMediaTitle = file.name.replace(/\.[^/.]+$/, '');
      }
      const reader = new FileReader();
      reader.onload = (ev) => {
        uploadMediaPreview = ev.target?.result as string;
      };
      reader.readAsDataURL(file);
    }
  }

  function saveCustomMedia() {
    const dataUrl = uploadMediaPreview || uploadMediaUrl.trim();
    if (!dataUrl) {
      uploadMediaError = 'Please choose an image file or provide an image URL';
      return;
    }
    const id = `custom-media-${Date.now()}`;
    const newItem: MediaItem = {
      id,
      title: uploadMediaTitle.trim() || 'Custom Backdrop',
      category: 'custom',
      themeValue: dataUrl,
      thumbnailStyle: `background: url("${dataUrl}") center/cover no-repeat;`,
      description: 'Custom uploaded backdrop',
      isCustom: true,
      dateAdded: Date.now()
    };
    customMedia = [newItem, ...customMedia];
    try {
      localStorage.setItem(STORAGE_KEY_CUSTOM_MEDIA, JSON.stringify(customMedia));
    } catch (e) {
      console.warn('Failed to save custom media to localStorage:', e);
    }
    showUploadModal = false;
    uploadMediaTitle = '';
    uploadMediaPreview = null;
    uploadMediaUrl = '';
    uploadMediaError = null;
  }

  function deleteCustomMedia(id: string) {
    customMedia = customMedia.filter(m => m.id !== id);
    try {
      localStorage.setItem(STORAGE_KEY_CUSTOM_MEDIA, JSON.stringify(customMedia));
    } catch (e) {
      console.warn('Failed to persist custom media deletion:', e);
    }
  }
</script>

<div class="h-screen w-screen flex flex-col font-sans antialiased select-none transition-colors duration-200 {effectiveTheme === 'light' ? 'theme-light' : 'theme-dark bg-[#09090b] text-[#f4f4f5]'}">
  <!-- TOP HEADER / STATUS BAR -->
  <header class="h-14 border-b border-neutral-800 px-5 flex items-center justify-between bg-[#0e0e11] shrink-0">
    <!-- Brand & Mic Status -->
    <div class="flex items-center gap-3">
      <div class="flex items-center gap-2">
        <span class="font-bold tracking-wider text-sm uppercase text-neutral-100">RELIEVE</span>
        <span class="text-[10px] font-mono tracking-wider px-1.5 py-0.5 rounded bg-neutral-800 text-neutral-400 border border-neutral-700/50">v0.1</span>
      </div>

      <div class="h-4 w-px bg-neutral-800 mx-1"></div>

      <!-- Speech Engine Setup Indicator (Only visible if downloading on first launch) -->
      {#if modelDownloading}
        <div class="flex items-center gap-2 px-2.5 py-1 rounded bg-amber-950/40 border border-amber-800/70 text-amber-300 text-xs font-mono">
          <DownloadCloud class="w-3.5 h-3.5 animate-bounce text-amber-400" />
          <span>Setting up Speech Engine: {downloadPercent}%</span>
          <div class="w-12 h-1.5 bg-neutral-800 rounded-full overflow-hidden">
            <div class="h-full bg-amber-400 transition-all duration-150" style="width: {downloadPercent}%"></div>
          </div>
        </div>
        <div class="h-4 w-px bg-neutral-800 mx-1"></div>
      {/if}

      <!-- Microphone Selector & Trigger -->
      <div class="flex items-center gap-2">
        <button
          onclick={toggleAudio}
          class="flex items-center gap-2 px-3 py-1.5 rounded text-xs font-medium transition-colors border {audioActive ? 'bg-emerald-950/40 text-emerald-300 border-emerald-800/80 hover:bg-emerald-900/40' : 'bg-neutral-900 text-neutral-400 border-neutral-800 hover:text-neutral-200 hover:border-neutral-700'}"
          title={audioActive ? 'Click to pause mic' : 'Click to start mic'}
        >
          {#if audioActive}
            <Mic class="w-3.5 h-3.5 text-emerald-400 animate-pulse" />
            <span>Listening</span>
          {:else}
            <MicOff class="w-3.5 h-3.5" />
            <span>Mic Off</span>
          {/if}
        </button>

        <!-- Audio VU Meter Bar -->
        <div class="w-20 h-2 bg-neutral-900 rounded-full overflow-hidden border border-neutral-800 flex items-center px-0.5" title="Microphone Level ({selectedDevice || 'Default device'}) — Change in Settings">
          <div
            class="h-1 rounded-full transition-all duration-75 {audioLevel > 0.6 ? 'bg-amber-400' : 'bg-emerald-400'}"
            style="width: {Math.max(4, Math.round(audioLevel * 100))}%"
          ></div>
        </div>
      </div>
    </div>

    <!-- Mode, Translation & Projector Controls -->
    <div class="flex items-center gap-3">
      <!-- Auto-Pilot / Co-Pilot Toggle -->
      <button
        onclick={toggleAutoProject}
        class="flex items-center gap-1.5 px-3 py-1.5 rounded text-xs font-medium transition-colors border {autoProject ? 'bg-neutral-100 text-neutral-950 border-neutral-100' : 'bg-neutral-900 text-neutral-400 border-neutral-800 hover:text-neutral-200 hover:border-neutral-700'}"
        title="Toggle between Assisted Countdown and Instant Push"
      >
        <Sparkles class="w-3.5 h-3.5" />
        <span>{autoProject ? 'Auto-Pilot' : 'Co-Pilot Mode'}</span>
      </button>

      <!-- Translation Selector -->
      <div class="flex items-center rounded border border-neutral-800 bg-neutral-900 p-0.5 text-xs">
        <button
          onclick={() => changeTranslation('KJV')}
          class="px-2.5 py-1 rounded font-medium transition-colors {activeTranslation === 'KJV' ? 'bg-neutral-800 text-neutral-100' : 'text-neutral-500 hover:text-neutral-300'}"
        >
          KJV
        </button>
        <button
          onclick={() => changeTranslation('WEB')}
          class="px-2.5 py-1 rounded font-medium transition-colors {activeTranslation === 'WEB' ? 'bg-neutral-800 text-neutral-100' : 'text-neutral-500 hover:text-neutral-300'}"
        >
          WEB
        </button>
      </div>

      <div class="h-4 w-px bg-neutral-800"></div>

      <!-- Quick Projector Action Buttons -->
      <div class="flex items-center gap-1.5">
        <button
          onclick={clearSlide}
          class="px-2.5 py-1.5 rounded text-xs font-medium border transition-colors {projectorClear ? 'bg-neutral-800 text-amber-300 border-amber-800/60' : 'bg-neutral-900 text-neutral-400 border-neutral-800 hover:text-neutral-200 hover:border-neutral-700'}"
          title="Clear verse text from screen (Key: C)"
        >
          Clear (C)
        </button>

        <button
          onclick={toggleBlackout}
          class="px-2.5 py-1.5 rounded text-xs font-medium border transition-colors {projectorBlackout ? 'bg-neutral-800 text-red-400 border-red-800/80' : 'bg-neutral-900 text-neutral-400 border-neutral-800 hover:text-neutral-200 hover:border-neutral-700'}"
          title="Blackout screen output completely (Key: B)"
        >
          Blackout (B)
        </button>

        <!-- Projector Window Trigger -->
        <button
          onclick={toggleProjector}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded text-xs font-medium transition-colors border {projectorOpen ? 'bg-neutral-200 text-neutral-900 border-neutral-200' : 'bg-neutral-900 text-neutral-300 border-neutral-700 hover:border-neutral-600'}"
        >
          <Monitor class="w-3.5 h-3.5" />
          <span>{projectorOpen ? 'Projector Active' : 'Open Projector'}</span>
        </button>

        <div class="h-4 w-px bg-neutral-800 mx-0.5"></div>

        <!-- Interface Light/Dark Mode Quick Toggle -->
        <button
          onclick={toggleAppTheme}
          class="flex items-center justify-center w-8 h-8 rounded border transition-colors {effectiveTheme === 'light' ? 'bg-stone-100 hover:bg-stone-200 text-amber-600 border-stone-300 shadow-sm' : 'bg-neutral-900 hover:bg-neutral-800 text-neutral-300 border-neutral-700'}"
          title={effectiveTheme === 'light' ? 'Switch to Dark Mode (Currently Light)' : 'Switch to Light Mode (Currently Dark)'}
          aria-label="Toggle Interface Theme"
        >
          {#if effectiveTheme === 'light'}
            <Sun class="w-4 h-4 text-amber-500" />
          {:else}
            <Moon class="w-4 h-4 text-neutral-300" />
          {/if}
        </button>
      </div>
    </div>
  </header>

  <!-- MAIN OPERATOR WORKSPACE -->
  <main class="flex-1 grid grid-cols-12 overflow-hidden">
    <!-- LEFT PANEL: SPEECH RADAR OR SONGS WORKSPACE (7 Cols) -->
    <div class="col-span-7 border-r border-neutral-800 flex flex-col h-full bg-[#09090b] overflow-hidden">
      {#if activeDockView === 'scripture'}
        <!-- Live Speech Section -->
        <div class="p-5 border-b border-neutral-800 flex flex-col gap-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <span class="w-2 h-2 rounded-full {audioActive ? 'bg-emerald-400 animate-pulse' : 'bg-neutral-600'}"></span>
              <span class="text-xs font-semibold tracking-wide uppercase text-neutral-400">Live Voice Transcription</span>
            </div>
            <div class="flex items-center gap-2 text-[11px] font-mono text-neutral-500">
              {#if audioActive}
                <span class="text-emerald-400 flex items-center gap-1.5">
                  <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-ping"></span>
                  VAD Active (16kHz Whisper)
                </span>
              {:else}
                <span class="text-neutral-500">Mic Inactive</span>
              {/if}
            </div>
          </div>

          <!-- Live Streaming Transcript Box -->
          <div class="min-h-[72px] max-h-[100px] overflow-y-auto p-3.5 rounded bg-neutral-950/70 border border-neutral-800/80 text-sm leading-relaxed text-neutral-200">
            {#if liveTranscript}
              <p class="font-normal text-neutral-100">{liveTranscript}</p>
            {:else if audioActive}
              <p class="text-neutral-500 italic flex items-center gap-2">
                <span class="inline-block w-1.5 h-1.5 rounded-full bg-emerald-400/80 animate-pulse"></span>
                Listening for pastor's speech...
              </p>
            {:else}
              <p class="text-neutral-600 italic">Waiting for microphone activation...</p>
            {/if}
          </div>
        </div>

        <!-- SCRIPTURE INTENT CARD (The Star Feature) -->
        <div class="flex-1 p-5 flex flex-col gap-3 overflow-y-auto">
          <div class="flex items-center justify-between">
            <span class="text-xs font-semibold tracking-wide uppercase text-neutral-400">Detected Scripture Intent</span>
            {#if detectedPassage}
              <span class="text-[11px] font-mono px-2 py-0.5 rounded bg-emerald-950/50 text-emerald-400 border border-emerald-800/60 flex items-center gap-1">
                <CheckCircle2 class="w-3 h-3" />
                High Confidence Match
              </span>
            {/if}
          </div>

          {#if detectedPassage}
            <!-- The Minimalist Intent Card -->
            <div class="rounded-lg border {effectiveTheme === 'light' ? 'bg-white border-stone-200 shadow-md' : 'border-neutral-700 bg-neutral-900/90 shadow-xl'} p-5 flex flex-col gap-4 transition-all">
              <!-- Header of card -->
              <div class="flex items-start justify-between">
                <div>
                  <h3 class="text-lg font-bold {effectiveTheme === 'light' ? 'text-stone-900' : 'text-neutral-100'} flex items-center gap-2">
                    <span>{detectedPassage.reference}</span>
                    <span class="text-xs font-mono font-normal {effectiveTheme === 'light' ? 'text-stone-600 bg-stone-100 border border-stone-200' : 'text-neutral-400 bg-neutral-800'} px-1.5 py-0.5 rounded">
                      {detectedPassage.translation}
                    </span>
                  </h3>
                  <p class="text-xs {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'} mt-0.5">Spoken by pastor in live sermon feed</p>
                </div>

                <!-- Actions -->
                <div class="flex items-center gap-2">
                  <button
                    onclick={dismissDetected}
                    class="px-3 py-1.5 rounded text-xs font-medium {effectiveTheme === 'light' ? 'text-stone-600 border-stone-200 hover:bg-stone-100 hover:text-stone-900' : 'text-neutral-400 border-neutral-800 hover:bg-neutral-800 hover:text-neutral-200'} border transition-colors flex items-center gap-1"
                  >
                    <X class="w-3.5 h-3.5" />
                    <span>Dismiss (ESC)</span>
                  </button>
                  <button
                    onclick={confirmPresent}
                    class="px-4 py-1.5 rounded text-xs font-semibold {effectiveTheme === 'light' ? 'bg-stone-900 text-white hover:bg-stone-800' : 'bg-neutral-100 text-neutral-950 hover:bg-white'} transition-colors flex items-center gap-1.5 shadow-sm"
                  >
                    <Send class="w-3.5 h-3.5" />
                    <span>Present Now (SPACE)</span>
                  </button>
                </div>
              </div>

              <!-- Verse Body Preview -->
              <div class="p-3.5 rounded {effectiveTheme === 'light' ? 'bg-stone-50 border-stone-200 text-stone-800' : 'bg-neutral-950/80 border-neutral-800 text-neutral-300'} border text-sm leading-relaxed font-serif italic">
                "{detectedPassage.combined_text}"
              </div>

              <!-- Countdown Bar in Co-Pilot Mode -->
              {#if !autoProject}
                <div class="flex flex-col gap-1.5 pt-1">
                  <div class="flex items-center justify-between text-xs {effectiveTheme === 'light' ? 'text-stone-600' : 'text-neutral-400'} font-mono">
                    <span>Auto-projecting to audience...</span>
                    <span class="font-bold {effectiveTheme === 'light' ? 'text-stone-900' : 'text-neutral-200'}">{countdown}s</span>
                  </div>
                  <div class="w-full h-1.5 {effectiveTheme === 'light' ? 'bg-stone-200' : 'bg-neutral-800'} rounded-full overflow-hidden">
                    <div
                      class="h-full {effectiveTheme === 'light' ? 'bg-stone-900' : 'bg-neutral-200'} transition-all duration-100 ease-linear"
                      style="width: {(countdown / 3.0) * 100}%"
                    ></div>
                  </div>
                </div>
              {/if}
            </div>
          {:else}
            <!-- Empty Intent State -->
            <div class="flex-1 flex flex-col items-center justify-center border border-dashed {effectiveTheme === 'light' ? 'border-stone-300 bg-stone-50/60' : 'border-neutral-800/80 bg-neutral-950/20'} rounded-lg p-8 text-center">
              <div class="w-10 h-10 rounded-full {effectiveTheme === 'light' ? 'bg-white border-stone-200 text-amber-500 shadow-xs' : 'bg-neutral-900 border-neutral-800 text-neutral-500'} flex items-center justify-center mb-3 border">
                <Sparkles class="w-5 h-5" />
              </div>
              <p class="text-sm font-medium {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-400'}">Listening for spoken citations...</p>
              <p class="text-xs {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-600'} mt-1 max-w-sm">
                When the pastor mentions a scripture (e.g., "John 3:16" or "Romans 8:28"), it will be recognized, retrieved, and presented here.
              </p>
            </div>
          {/if}

          <!-- Recent Transcript History -->
          {#if transcriptHistory.length > 0}
            <div class="mt-auto pt-2 border-t border-neutral-800/60">
              <span class="text-[11px] font-semibold text-neutral-500 uppercase tracking-wider">Recent Speech History</span>
              <div class="flex flex-col gap-1 mt-1.5 max-h-24 overflow-y-auto text-xs text-neutral-500 font-mono">
                {#each transcriptHistory.slice(1, 4) as item}
                  <div class="truncate">• {item}</div>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {:else if activeDockView === 'songs'}
        {#if songStudioView === 'editor'}
          <!-- VS CODE IN-WORKSPACE SONG CREATOR & EDITOR -->
          <div class="flex flex-col h-full overflow-hidden {effectiveTheme === 'light' ? 'bg-[#f7f5f0]' : 'bg-[#09090c]'}">
            <!-- Breadcrumb Header -->
            <div class="p-4 border-b {effectiveTheme === 'light' ? 'border-stone-200 bg-white' : 'border-neutral-800/80 bg-[#0c0c10]'} flex items-center justify-between gap-3 shrink-0">
              <div class="flex items-center gap-2.5">
                <button
                  onclick={closeSongEditor}
                  class="p-1.5 rounded-lg {effectiveTheme === 'light' ? 'bg-stone-100 border-stone-200 text-stone-600 hover:text-stone-900 hover:bg-stone-200' : 'bg-neutral-900 border-neutral-800 text-neutral-400 hover:text-neutral-200'} border transition-colors"
                  title="Return to Song Library"
                >
                  <ArrowLeft class="w-4 h-4" />
                </button>
                <div>
                  <div class="flex items-center gap-1.5 text-[10px] font-mono uppercase tracking-wider {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'}">
                    <span>Songs</span>
                    <span>/</span>
                    <span class="text-amber-500">{editingSongId ? 'Edit Song' : 'New Song'}</span>
                  </div>
                  <h2 class="text-xs font-bold {effectiveTheme === 'light' ? 'text-stone-900' : 'text-neutral-100'}">
                    {editingSongId ? (newSongTitle || 'Untitled Song') : 'Create New Worship Song'}
                  </h2>
                </div>
              </div>

              <div class="flex items-center gap-2">
                <button
                  onclick={closeSongEditor}
                  class="px-3 py-1.5 rounded border {effectiveTheme === 'light' ? 'border-stone-200 text-stone-600 hover:text-stone-900 hover:bg-stone-100' : 'border-neutral-800 text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800'} text-xs font-medium transition-colors"
                >
                  Cancel
                </button>
                <button
                  onclick={saveSong}
                  disabled={!newSongTitle.trim() || parsedStanzas.length === 0}
                  class="px-4 py-1.5 rounded bg-amber-500 hover:bg-amber-400 disabled:opacity-40 disabled:hover:bg-amber-500 text-neutral-950 text-xs font-semibold transition-colors shadow-sm"
                >
                  {editingSongId ? 'Update Song' : 'Save to Library'}
                </button>
              </div>
            </div>

            <!-- Two-Column Editor Body -->
            <div class="p-5 grid grid-cols-12 gap-5 overflow-y-auto flex-1">
              <!-- Left: Form inputs & lyrics textarea (7 cols) -->
              <div class="col-span-7 flex flex-col gap-3">
                <div class="grid grid-cols-2 gap-3">
                  <div class="flex flex-col gap-1">
                    <label for="song-title-input" class="text-[10px] font-semibold {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-400'} uppercase tracking-wider">Song Title *</label>
                    <input
                      id="song-title-input"
                      type="text"
                      bind:value={newSongTitle}
                      placeholder="e.g. 10,000 Reasons"
                      class="w-full {effectiveTheme === 'light' ? 'bg-white border-stone-200 text-stone-900 placeholder-stone-400 focus:border-amber-500' : 'bg-neutral-950 border-neutral-800 text-neutral-200 focus:border-amber-600/70'} border text-xs rounded px-3 py-2 focus:outline-none"
                    />
                  </div>
                  <div class="flex flex-col gap-1">
                    <label for="song-author-input" class="text-[10px] font-semibold {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-400'} uppercase tracking-wider">Author / Artist</label>
                    <input
                      id="song-author-input"
                      type="text"
                      bind:value={newSongAuthor}
                      placeholder="e.g. Matt Redman"
                      class="w-full {effectiveTheme === 'light' ? 'bg-white border-stone-200 text-stone-900 placeholder-stone-400 focus:border-stone-400' : 'bg-neutral-950 border-neutral-800 text-neutral-200 focus:border-neutral-700'} border text-xs rounded px-3 py-2 focus:outline-none"
                    />
                  </div>
                </div>

                <div class="flex flex-col gap-1">
                  <label for="song-category-select" class="text-[10px] font-semibold {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-400'} uppercase tracking-wider">Category</label>
                  <select
                    id="song-category-select"
                    bind:value={newSongCategory}
                    class="w-full {effectiveTheme === 'light' ? 'bg-white border-stone-200 text-stone-900 focus:border-stone-400' : 'bg-neutral-950 border-neutral-800 text-neutral-200 focus:border-neutral-700'} border text-xs rounded px-3 py-2 focus:outline-none"
                  >
                    <option value="Worship">Worship</option>
                    <option value="Praise">Praise</option>
                    <option value="Hymn">Hymn</option>
                    <option value="Gospel">Gospel</option>
                  </select>
                </div>

                <div class="flex flex-col gap-1 flex-1">
                  <div class="flex items-center justify-between">
                    <label for="song-lyrics-textarea" class="text-[10px] font-semibold {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-400'} uppercase tracking-wider">Paste Raw Lyrics *</label>
                    <span class="text-[10px] {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'} font-mono">Auto-detects [Verse], [Chorus] or blank lines</span>
                  </div>
                  <textarea
                    id="song-lyrics-textarea"
                    bind:value={newSongRawLyrics}
                    oninput={handleLyricsInput}
                    rows="10"
                    placeholder="Paste lyrics here...&#10;&#10;[Verse 1]&#10;Bless the Lord O my soul&#10;O my soul&#10;Worship His holy name&#10;&#10;[Chorus]&#10;Sing like never before&#10;O my soul&#10;I'll worship Your holy name&#10;&#10;Or simply separate stanzas with empty blank lines!"
                    class="w-full {effectiveTheme === 'light' ? 'bg-white border-stone-200 text-stone-900 placeholder-stone-400 focus:border-amber-500' : 'bg-neutral-950 border-neutral-800 text-neutral-200 focus:border-amber-600/70'} border text-xs font-mono rounded p-3 focus:outline-none resize-none leading-relaxed"
                  ></textarea>
                </div>
              </div>

              <!-- Right: Live Slide Breakdown (5 cols) -->
              <div class="col-span-5 flex flex-col gap-2 border-l {effectiveTheme === 'light' ? 'border-stone-200' : 'border-neutral-800/80'} pl-5">
                <div class="flex items-center justify-between pb-1 border-b {effectiveTheme === 'light' ? 'border-stone-100' : 'border-neutral-800'}">
                  <span class="text-[10px] font-semibold {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-400'} uppercase tracking-wider">Live Slide Breakdown</span>
                  <span class="text-[10px] font-mono px-1.5 py-0.5 rounded {effectiveTheme === 'light' ? 'bg-stone-100 text-stone-700 border-stone-200' : 'bg-neutral-900 text-neutral-300 border-neutral-800'} border">
                    {parsedStanzas.length} Slide{parsedStanzas.length === 1 ? '' : 's'}
                  </span>
                </div>

                <div class="flex-1 overflow-y-auto flex flex-col gap-2.5 max-h-[380px] pr-1">
                  {#if parsedStanzas.length === 0}
                    <div class="h-full flex flex-col items-center justify-center p-6 text-center {effectiveTheme === 'light' ? 'text-stone-400' : 'text-neutral-500'} italic gap-2">
                      <Music class="w-8 h-8 opacity-30 text-amber-500" />
                      <p class="text-xs">Type or paste lyrics on the left.</p>
                      <p class="text-[11px] {effectiveTheme === 'light' ? 'text-stone-400' : 'text-neutral-600'}">Each tagged block or blank-line paragraph creates a projection slide.</p>
                    </div>
                  {:else}
                    {#each parsedStanzas as stanza, idx}
                      <div class="p-3 rounded-lg {effectiveTheme === 'light' ? 'bg-white border-stone-200 shadow-xs' : 'bg-neutral-950/80 border-neutral-800'} border flex flex-col gap-1.5 shadow-sm">
                        <div class="flex items-center justify-between">
                          <span class="text-[11px] font-bold text-amber-500 font-mono">{stanza.label}</span>
                          <span class="text-[9px] font-mono {effectiveTheme === 'light' ? 'text-stone-400' : 'text-neutral-500'}">Slide {idx + 1}</span>
                        </div>
                        <p class="text-xs {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-300'} font-serif italic whitespace-pre-line leading-relaxed">
                          {stanza.text}
                        </p>
                      </div>
                    {/each}
                  {/if}
                </div>
              </div>
            </div>
          </div>
        {:else}
          <!-- SONGS & WORSHIP LYRICS STUDIO -->
          <div class="flex flex-col h-full overflow-hidden {effectiveTheme === 'light' ? 'bg-[#f7f5f0]' : 'bg-[#09090c]'}">
            <!-- Top Bar: Search & Title -->
            <div class="p-4 border-b {effectiveTheme === 'light' ? 'border-stone-200 bg-white' : 'border-neutral-800/80 bg-[#0c0c10]'} flex items-center justify-between gap-3 shrink-0">
              <div class="flex items-center gap-2">
                <div class="p-1.5 rounded-lg {effectiveTheme === 'light' ? 'bg-amber-50 border-amber-200 text-amber-600' : 'bg-amber-950/40 border-amber-800/60 text-amber-400'} border">
                  <Music class="w-4 h-4" />
                </div>
                <div>
                  <h2 class="text-xs font-bold {effectiveTheme === 'light' ? 'text-stone-900' : 'text-neutral-100'} uppercase tracking-wider">Songs & Worship Library</h2>
                  <p class="text-[10px] {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'}">{filteredSongs.length} songs available • Click any stanza to project live</p>
                </div>
              </div>

              <!-- Search input & New Song button -->
              <div class="flex items-center gap-2.5">
                <div class="relative w-56">
                  <Search class="w-3.5 h-3.5 {effectiveTheme === 'light' ? 'text-stone-400' : 'text-neutral-500'} absolute left-2.5 top-1/2 -translate-y-1/2" />
                  <input
                    type="text"
                    bind:value={songSearch}
                    placeholder="Search songs or lyrics..."
                    class="w-full pl-8 pr-3 py-1.5 {effectiveTheme === 'light' ? 'bg-stone-50 border-stone-200 text-stone-900 placeholder-stone-400 focus:border-amber-500' : 'bg-neutral-900 border-neutral-800 text-neutral-200 placeholder-neutral-500 focus:border-neutral-700'} border rounded text-xs focus:outline-none"
                  />
                  {#if songSearch}
                    <button
                      onclick={() => songSearch = ''}
                      class="absolute right-2 top-1/2 -translate-y-1/2 {effectiveTheme === 'light' ? 'text-stone-400 hover:text-stone-600' : 'text-neutral-500 hover:text-neutral-300'}"
                    >
                      <X class="w-3 h-3" />
                    </button>
                  {/if}
                </div>

                <button
                  onclick={openNewSongModal}
                  class="px-3 py-1.5 rounded bg-amber-500 hover:bg-amber-400 text-neutral-950 text-xs font-semibold flex items-center gap-1.5 transition-colors shadow-sm shrink-0"
                >
                  <Plus class="w-3.5 h-3.5" />
                  <span>New Song</span>
                </button>
              </div>
            </div>

            <!-- Split Body: Song Directory + Stanzas -->
            <div class="flex-1 grid grid-cols-12 overflow-hidden">
              <!-- Song Directory (5 cols) -->
              <div class="col-span-5 border-r {effectiveTheme === 'light' ? 'bg-stone-50/50 border-stone-200' : 'bg-[#09090b] border-neutral-800/80'} p-2 overflow-y-auto flex flex-col gap-1">
                {#each filteredSongs as song}
                  <div
                    role="button"
                    tabindex="0"
                    onclick={() => selectedSongId = song.id}
                    onkeydown={(e) => e.key === 'Enter' && (selectedSongId = song.id)}
                    class="p-2.5 rounded-lg text-left transition-all border flex flex-col gap-0.5 group cursor-pointer {selectedSongId === song.id ? (effectiveTheme === 'light' ? 'bg-white border-amber-500 ring-1 ring-amber-400/40 text-stone-900 shadow-sm' : 'bg-neutral-800/80 border-neutral-700 text-neutral-100 shadow-sm') : (effectiveTheme === 'light' ? 'bg-white/80 border-stone-200/80 hover:bg-white text-stone-700 hover:text-stone-900 shadow-xs' : 'bg-neutral-900/30 border-transparent hover:bg-neutral-900/70 text-neutral-400 hover:text-neutral-200')}"
                  >
                    <div class="flex items-center justify-between">
                      <span class="text-xs font-semibold {effectiveTheme === 'light' ? 'text-stone-900 group-hover:text-amber-600' : 'text-neutral-200 group-hover:text-white'} truncate">{song.title}</span>
                      <div class="flex items-center gap-1">
                        {#if song.id.startsWith('custom-')}
                          <button
                            onclick={(e) => { e.stopPropagation(); openEditSongModal(song); }}
                            class="p-1 rounded text-neutral-500 hover:text-neutral-200 hover:bg-neutral-700/60 transition-colors"
                            title="Edit Song"
                          >
                            <Edit2 class="w-3 h-3" />
                          </button>
                          <button
                            onclick={(e) => { e.stopPropagation(); deleteCustomSong(song.id); }}
                            class="p-1 rounded text-neutral-500 hover:text-red-400 hover:bg-neutral-700/60 transition-colors"
                            title="Delete Song"
                          >
                            <Trash2 class="w-3 h-3" />
                          </button>
                        {/if}
                        <span class="text-[9px] font-mono px-1.5 py-0.5 rounded shrink-0 {selectedSongId === song.id ? (effectiveTheme === 'light' ? 'bg-amber-100 text-amber-800 font-semibold' : 'bg-neutral-700 text-neutral-200') : (effectiveTheme === 'light' ? 'bg-stone-100 text-stone-600' : 'bg-neutral-800/80 text-neutral-500')}">
                          {song.category}
                        </span>
                      </div>
                    </div>
                    <span class="text-[10px] {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'} truncate italic font-serif">{song.author}</span>
                  </div>
                {/each}
              </div>

              <!-- Stanzas / Projection Panel (7 cols) -->
              <div class="col-span-7 p-4 overflow-y-auto flex flex-col gap-3 {effectiveTheme === 'light' ? 'bg-white' : 'bg-[#0a0a0e]'}">
                <!-- Selected Song Meta -->
                <div class="border-b {effectiveTheme === 'light' ? 'border-stone-200' : 'border-neutral-800'} pb-3 flex items-start justify-between">
                  <div>
                    <h3 class="text-base font-bold {effectiveTheme === 'light' ? 'text-stone-900' : 'text-neutral-100'}">{selectedSong.title}</h3>
                    <p class="text-xs {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-400'} font-serif italic mt-0.5">{selectedSong.author} • {selectedSong.stanzas.length} Stanzas</p>
                  </div>
                  <span class="text-[10px] font-mono {effectiveTheme === 'light' ? 'bg-stone-100 text-stone-600 border-stone-200' : 'bg-neutral-900 text-neutral-500 border-neutral-800'} uppercase tracking-wider px-2 py-0.5 rounded border">
                    {selectedSong.category}
                  </span>
                </div>

                <!-- Stanza Cards -->
                <div class="flex flex-col gap-2.5">
                  {#each selectedSong.stanzas as stanza, idx}
                    {@const isLive = activeSongStanzaKey === `${selectedSong.id}-${stanza.label}` && !projectorClear && !projectorBlackout}
                    <button
                      onclick={() => presentSongStanza(selectedSong, stanza, idx)}
                      class="p-3.5 rounded-lg border text-left transition-all flex flex-col gap-2 group {isLive ? (effectiveTheme === 'light' ? 'bg-emerald-50 border-emerald-500 text-emerald-950 shadow-md' : 'bg-emerald-950/20 border-emerald-700/80 shadow-lg') : (effectiveTheme === 'light' ? 'bg-white border-stone-200 hover:border-stone-300 hover:bg-stone-50/80 shadow-xs' : 'bg-neutral-900/50 border-neutral-800 hover:border-neutral-700 hover:bg-neutral-900')}"
                    >
                      <div class="flex items-center justify-between">
                        <div class="flex items-center gap-2">
                          <span class="text-xs font-bold {isLive ? (effectiveTheme === 'light' ? 'text-emerald-700' : 'text-emerald-400') : (effectiveTheme === 'light' ? 'text-stone-800' : 'text-neutral-300')}">
                            {stanza.label}
                          </span>
                          {#if isLive}
                            <span class="px-1.5 py-0.5 rounded {effectiveTheme === 'light' ? 'bg-emerald-100 text-emerald-800 border-emerald-300' : 'bg-emerald-950 text-emerald-400 border-emerald-800/80'} border text-[10px] font-mono flex items-center gap-1 animate-pulse">
                              <span class="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
                              ON AIR
                            </span>
                          {/if}
                        </div>
                        <span class="text-[10px] font-mono {effectiveTheme === 'light' ? 'text-stone-400 group-hover:text-stone-700' : 'text-neutral-500 group-hover:text-neutral-300'} flex items-center gap-1">
                          <Send class="w-3 h-3" />
                          Project
                        </span>
                      </div>

                      <p class="text-xs {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-300'} font-serif italic whitespace-pre-line leading-relaxed">
                        {stanza.text}
                      </p>
                    </button>
                  {/each}
                </div>
              </div>
            </div>
          </div>
        {/if}
      {:else if activeDockView === 'media'}
        <!-- MEDIA & MOTION BACKDROPS STUDIO -->
        <div class="flex flex-col h-full overflow-hidden {effectiveTheme === 'light' ? 'bg-[#f7f5f0]' : 'bg-[#09090c]'}">
          <!-- Media Studio Top Header -->
          <div class="p-4 border-b {effectiveTheme === 'light' ? 'border-stone-200 bg-white' : 'border-neutral-800/80 bg-[#0c0c10]'} flex flex-col md:flex-row md:items-center justify-between gap-3 shrink-0">
            <div class="flex items-center gap-2.5">
              <div class="p-1.5 rounded-lg {effectiveTheme === 'light' ? 'bg-rose-50 border-rose-200 text-rose-600' : 'bg-rose-950/40 border-rose-800/60 text-rose-400'} border">
                <Image class="w-4 h-4" />
              </div>
              <div>
                <div class="flex items-center gap-2">
                  <h2 class="text-xs font-bold {effectiveTheme === 'light' ? 'text-stone-900' : 'text-neutral-100'} uppercase tracking-wider">Media & Motion Backdrops Studio</h2>
                  <span class="px-1.5 py-0.5 rounded text-[10px] font-mono font-medium {effectiveTheme === 'light' ? 'bg-stone-100 text-stone-600' : 'bg-neutral-800 text-neutral-400'}">
                    {allMediaItems.length} Backdrops
                  </span>
                </div>
                <p class="text-[10px] {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'}">
                  Curated ambient worship gradients, stage lighting glows, and custom church presentation backdrops
                </p>
              </div>
            </div>

            <div class="flex items-center gap-2">
              <!-- Search Input -->
              <div class="relative">
                <Search class="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-400" />
                <input
                  type="text"
                  bind:value={mediaSearchQuery}
                  placeholder="Search backdrops..."
                  class="w-44 pl-8 pr-3 py-1.5 rounded text-xs {effectiveTheme === 'light' ? 'bg-stone-100 border-stone-300 text-stone-900 focus:bg-white' : 'bg-neutral-900 border-neutral-700/80 text-neutral-200 focus:bg-neutral-800'} border focus:outline-hidden"
                />
                {#if mediaSearchQuery}
                  <button onclick={() => mediaSearchQuery = ''} class="absolute right-2 top-1/2 -translate-y-1/2 text-neutral-400 hover:text-neutral-200">
                    <X class="w-3 h-3" />
                  </button>
                {/if}
              </div>

              <!-- Upload Button -->
              <button
                onclick={() => { showUploadModal = true; uploadMediaError = null; }}
                class="px-3 py-1.5 rounded bg-rose-600 hover:bg-rose-500 text-white text-xs font-medium transition-colors flex items-center gap-1.5 shadow-xs"
              >
                <Upload class="w-3.5 h-3.5" />
                <span>Upload Media</span>
              </button>

              <!-- Back to Console -->
              <button
                onclick={() => activeDockView = 'scripture'}
                class="px-2.5 py-1.5 rounded {effectiveTheme === 'light' ? 'bg-stone-100 border-stone-200 hover:bg-stone-200 text-stone-700' : 'bg-neutral-900 border-neutral-800 hover:bg-neutral-800 text-neutral-300'} border text-xs font-medium transition-colors flex items-center gap-1.5 shadow-xs"
              >
                <span>← Console</span>
              </button>
            </div>
          </div>

          <!-- Secondary Toolbar: Category Tabs & Live Status Bar -->
          <div class="px-4 py-2 border-b {effectiveTheme === 'light' ? 'border-stone-200/80 bg-stone-50/70' : 'border-neutral-850 bg-[#0c0c10]/60'} flex flex-wrap items-center justify-between gap-2 shrink-0">
            <!-- Filter Tabs -->
            <div class="flex items-center gap-1">
              {#each [
                { id: 'all', label: 'All Backdrops', count: allMediaItems.length },
                { id: 'motion', label: 'Motion & Glow', count: allMediaItems.filter(m => m.category === 'motion').length },
                { id: 'stage', label: 'Stage Presets', count: allMediaItems.filter(m => m.category === 'stage').length },
                { id: 'custom', label: 'Custom Uploads', count: customMedia.length }
              ] as tab}
                <button
                  onclick={() => mediaCategoryFilter = tab.id as any}
                  class="px-2.5 py-1 rounded text-xs transition-colors flex items-center gap-1.5 {mediaCategoryFilter === tab.id ? (effectiveTheme === 'light' ? 'bg-white border-stone-300 text-stone-900 font-semibold shadow-xs' : 'bg-neutral-800 border-neutral-700 text-neutral-100 font-semibold shadow-xs') : (effectiveTheme === 'light' ? 'text-stone-600 hover:text-stone-900 hover:bg-stone-100' : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800/40')} border border-transparent"
                >
                  <span>{tab.label}</span>
                  <span class="text-[10px] px-1 rounded-full {mediaCategoryFilter === tab.id ? (effectiveTheme === 'light' ? 'bg-stone-100 text-stone-800' : 'bg-neutral-700 text-neutral-200') : (effectiveTheme === 'light' ? 'bg-stone-200/60 text-stone-500' : 'bg-neutral-850 text-neutral-500')}">
                    {tab.count}
                  </span>
                </button>
              {/each}
            </div>

            <!-- Live Stage Theme Info Pill -->
            <div class="flex items-center gap-2 text-xs">
              <span class="text-[11px] {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-400'}">Active Stage Backdrop:</span>
              <span class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full {effectiveTheme === 'light' ? 'bg-emerald-50 text-emerald-800 border-emerald-200' : 'bg-emerald-950/60 text-emerald-300 border-emerald-800/80'} border font-mono text-[11px] font-semibold">
                <span class="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse"></span>
                {allMediaItems.find(m => m.themeValue === projectorTheme)?.title || (projectorTheme.startsWith('data:') || projectorTheme.startsWith('http') || projectorTheme.startsWith('url(') ? 'Custom Upload' : projectorTheme)}
              </span>
            </div>
          </div>

          <!-- Media Cards Gallery Grid -->
          <div class="flex-1 overflow-y-auto p-4 md:p-6">
            {#if filteredMediaItems.length === 0}
              <div class="h-64 flex flex-col items-center justify-center text-center p-6 rounded-xl border border-dashed {effectiveTheme === 'light' ? 'border-stone-300 bg-white/60' : 'border-neutral-800 bg-neutral-900/30'}">
                <div class="p-3 rounded-full {effectiveTheme === 'light' ? 'bg-stone-100 text-stone-500' : 'bg-neutral-800 text-neutral-400'} mb-3">
                  <Image class="w-6 h-6" />
                </div>
                <h3 class="text-sm font-semibold {effectiveTheme === 'light' ? 'text-stone-800' : 'text-neutral-200'}">No backdrops found</h3>
                <p class="text-xs {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'} max-w-sm mt-1 mb-4">
                  {mediaCategoryFilter === 'custom' ? 'You have not uploaded any custom media yet. Upload high-res images, title slides, or worship loops.' : 'No media items match your search term.'}
                </p>
                {#if mediaCategoryFilter === 'custom'}
                  <button
                    onclick={() => showUploadModal = true}
                    class="px-3 py-1.5 rounded bg-rose-600 hover:bg-rose-500 text-white text-xs font-medium transition-colors flex items-center gap-1.5 shadow-xs"
                  >
                    <Upload class="w-3.5 h-3.5" />
                    <span>Upload Custom Media</span>
                  </button>
                {/if}
              </div>
            {:else}
              <div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 xl:grid-cols-4 gap-4">
                {#each filteredMediaItems as item (item.id)}
                  {@const isLiveTheme = projectorTheme === item.themeValue}
                  <div
                    class="rounded-xl border transition-all duration-200 overflow-hidden flex flex-col group {isLiveTheme ? (effectiveTheme === 'light' ? 'border-emerald-500 ring-2 ring-emerald-500/20 bg-white shadow-md' : 'border-emerald-600/80 ring-2 ring-emerald-500/20 bg-neutral-900 shadow-lg') : (effectiveTheme === 'light' ? 'border-stone-200 bg-white hover:border-stone-300 hover:shadow-md' : 'border-neutral-800 bg-neutral-900/80 hover:border-neutral-700 hover:shadow-lg')}"
                  >
                    <!-- 16:9 Aspect Video Preview Container -->
                    <div
                      class="w-full aspect-video relative overflow-hidden flex items-center justify-center select-none"
                      style="{item.thumbnailStyle}"
                    >
                      <!-- Simulated Ambient Glow inside thumbnail for Motion Themes -->
                      {#if item.id === 'celestial-violet'}
                        <div class="absolute -top-1/4 -left-1/4 w-3/4 h-3/4 rounded-full bg-purple-500/25 blur-xl pointer-events-none"></div>
                        <div class="absolute top-1/4 -right-1/4 w-3/4 h-3/4 rounded-full bg-fuchsia-500/25 blur-xl pointer-events-none"></div>
                      {:else if item.id === 'emerald-sanctuary'}
                        <div class="absolute -top-1/4 -left-1/4 w-3/4 h-3/4 rounded-full bg-emerald-500/25 blur-xl pointer-events-none"></div>
                        <div class="absolute top-1/4 -right-1/4 w-3/4 h-3/4 rounded-full bg-teal-500/25 blur-xl pointer-events-none"></div>
                      {:else if item.id === 'crimson-majesty'}
                        <div class="absolute -top-1/4 -left-1/4 w-3/4 h-3/4 rounded-full bg-rose-500/25 blur-xl pointer-events-none"></div>
                        <div class="absolute top-1/4 -right-1/4 w-3/4 h-3/4 rounded-full bg-red-600/25 blur-xl pointer-events-none"></div>
                      {:else if item.id === 'oceanic-azure'}
                        <div class="absolute -top-1/4 -left-1/4 w-3/4 h-3/4 rounded-full bg-cyan-500/25 blur-xl pointer-events-none"></div>
                        <div class="absolute top-1/4 -right-1/4 w-3/4 h-3/4 rounded-full bg-teal-500/25 blur-xl pointer-events-none"></div>
                      {:else if item.id === 'golden-sunset'}
                        <div class="absolute -top-1/4 -left-1/4 w-3/4 h-3/4 rounded-full bg-orange-500/25 blur-xl pointer-events-none"></div>
                        <div class="absolute top-1/4 -right-1/4 w-3/4 h-3/4 rounded-full bg-amber-500/25 blur-xl pointer-events-none"></div>
                      {:else if item.id === 'atmospheric-blue'}
                        <div class="absolute -top-1/4 -left-1/4 w-3/4 h-3/4 rounded-full bg-cyan-500/25 blur-xl pointer-events-none"></div>
                        <div class="absolute top-1/4 -right-1/4 w-3/4 h-3/4 rounded-full bg-blue-600/25 blur-xl pointer-events-none"></div>
                      {/if}

                      <!-- Top Overlay Badges -->
                      <div class="absolute top-2 left-2 right-2 flex items-center justify-between pointer-events-none z-10">
                        {#if isLiveTheme}
                          <span class="px-2 py-0.5 rounded-full bg-emerald-500/90 text-white text-[10px] font-bold tracking-wider uppercase flex items-center gap-1 shadow-md backdrop-blur-xs">
                            <span class="w-1.5 h-1.5 rounded-full bg-white animate-pulse"></span>
                            Live Backdrop
                          </span>
                        {:else}
                          <span></span>
                        {/if}

                        <span class="px-1.5 py-0.5 rounded text-[9px] font-mono uppercase font-semibold {effectiveTheme === 'light' ? 'bg-black/60 text-white' : 'bg-black/70 text-neutral-300'} backdrop-blur-xs">
                          {item.category}
                        </span>
                      </div>

                      <!-- Hover Actions Overlay -->
                      <div class="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center gap-2 p-3 z-20 backdrop-blur-xs">
                        <button
                          onclick={() => setTheme(item.themeValue)}
                          class="px-2.5 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-[11px] font-semibold transition-transform transform hover:scale-105 flex items-center gap-1.5 shadow-md"
                          title="Apply this visual theme as the stage backdrop behind scriptures or lyrics"
                        >
                          <Palette class="w-3.5 h-3.5" />
                          <span>Set Backdrop</span>
                        </button>

                        <button
                          onclick={() => projectMediaSlide(item)}
                          class="px-2.5 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white text-[11px] font-semibold transition-transform transform hover:scale-105 flex items-center gap-1.5 shadow-md"
                          title="Present this backdrop full screen on the live projector"
                        >
                          <Play class="w-3.5 h-3.5" />
                          <span>Project Slide</span>
                        </button>
                      </div>
                    </div>

                    <!-- Card Body & Controls -->
                    <div class="p-3 flex flex-col justify-between flex-1 gap-2.5">
                      <div>
                        <div class="flex items-center justify-between">
                          <h4 class="text-xs font-bold {effectiveTheme === 'light' ? 'text-stone-900' : 'text-neutral-100'} truncate">
                            {item.title}
                          </h4>
                          {#if item.isCustom}
                            <button
                              onclick={() => deleteCustomMedia(item.id)}
                              class="text-neutral-400 hover:text-red-500 transition-colors p-1 rounded hover:bg-neutral-800/40"
                              title="Delete custom media"
                            >
                              <Trash2 class="w-3 h-3" />
                            </button>
                          {/if}
                        </div>
                        <p class="text-[10px] {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-400'} line-clamp-2 mt-0.5 leading-relaxed">
                          {item.description}
                        </p>
                      </div>

                      <!-- Quick Action Strip -->
                      <div class="flex items-center gap-1.5 pt-2 border-t {effectiveTheme === 'light' ? 'border-stone-100' : 'border-neutral-800/60'}">
                        <button
                          onclick={() => setTheme(item.themeValue)}
                          class="flex-1 py-1 px-2 rounded text-[11px] font-medium transition-colors flex items-center justify-center gap-1.5 {isLiveTheme ? (effectiveTheme === 'light' ? 'bg-emerald-100 text-emerald-800' : 'bg-emerald-950/60 text-emerald-300 border border-emerald-800/60') : (effectiveTheme === 'light' ? 'bg-stone-100 hover:bg-stone-200 text-stone-700' : 'bg-neutral-800 hover:bg-neutral-750 text-neutral-200')}"
                        >
                          <Palette class="w-3 h-3 text-emerald-500" />
                          <span>{isLiveTheme ? 'Active Backdrop' : 'Set Backdrop'}</span>
                        </button>

                        <button
                          onclick={() => projectMediaSlide(item)}
                          class="py-1 px-2.5 rounded text-[11px] font-medium transition-colors flex items-center justify-center gap-1 {effectiveTheme === 'light' ? 'bg-stone-100 hover:bg-indigo-50 hover:text-indigo-600 text-stone-700' : 'bg-neutral-800 hover:bg-indigo-950/60 hover:text-indigo-300 text-neutral-200'}"
                          title="Project Full Slide"
                        >
                          <Monitor class="w-3 h-3 text-indigo-400" />
                        </button>
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        </div>

        <!-- Upload Media Modal -->
        {#if showUploadModal}
          <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs">
            <div class="w-full max-w-md rounded-2xl {effectiveTheme === 'light' ? 'bg-white border-stone-200 shadow-2xl' : 'bg-[#121216] border-neutral-800 shadow-2xl'} border p-5 flex flex-col gap-4">
              <div class="flex items-center justify-between pb-3 border-b {effectiveTheme === 'light' ? 'border-stone-100' : 'border-neutral-800'}">
                <div class="flex items-center gap-2">
                  <div class="p-1 rounded-lg bg-rose-500/20 text-rose-500">
                    <Upload class="w-4 h-4" />
                  </div>
                  <h3 class="text-sm font-bold {effectiveTheme === 'light' ? 'text-stone-900' : 'text-neutral-100'}">
                    Upload Custom Stage Backdrop
                  </h3>
                </div>
                <button
                  onclick={() => showUploadModal = false}
                  class="text-neutral-400 hover:text-neutral-200 transition-colors"
                >
                  <X class="w-4 h-4" />
                </button>
              </div>

              {#if uploadMediaError}
                <div class="px-3 py-2 rounded-lg bg-red-500/10 border border-red-500/30 text-red-500 text-xs">
                  {uploadMediaError}
                </div>
              {/if}

              <!-- Media Title Input -->
              <div class="flex flex-col gap-1.5">
                <label for="media-title-input" class="text-[11px] font-semibold {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-300'} uppercase tracking-wider">
                  Backdrop Title
                </label>
                <input
                  id="media-title-input"
                  type="text"
                  bind:value={uploadMediaTitle}
                  placeholder="e.g. Sunday Morning Motion Loop, Series Graphic"
                  class="px-3 py-2 rounded-lg text-xs {effectiveTheme === 'light' ? 'bg-stone-50 border-stone-300 text-stone-900 focus:bg-white' : 'bg-neutral-900 border-neutral-700 text-neutral-100 focus:bg-neutral-850'} border focus:outline-hidden"
                />
              </div>

              <!-- Local Image File Dropzone -->
              <div class="flex flex-col gap-1.5">
                <span class="text-[11px] font-semibold {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-300'} uppercase tracking-wider">
                  Select Image File
                </span>
                <label
                  class="border-2 border-dashed rounded-xl p-4 flex flex-col items-center justify-center gap-2 cursor-pointer transition-colors {effectiveTheme === 'light' ? 'border-stone-300 bg-stone-50 hover:bg-stone-100' : 'border-neutral-700 bg-neutral-900/50 hover:bg-neutral-850'}"
                >
                  <Upload class="w-6 h-6 text-rose-500" />
                  <div class="text-center">
                    <span class="text-xs font-semibold {effectiveTheme === 'light' ? 'text-stone-800' : 'text-neutral-200'}">
                      Click to choose image file
                    </span>
                    <p class="text-[10px] {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'} mt-0.5">
                      PNG, JPG, WebP, GIF (up to 5MB)
                    </p>
                  </div>
                  <input
                    type="file"
                    accept="image/*"
                    onchange={handleMediaFileInput}
                    class="hidden"
                  />
                </label>
              </div>

              <!-- Or Direct Image URL -->
              <div class="flex flex-col gap-1.5">
                <label for="media-url-input" class="text-[11px] font-semibold {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-300'} uppercase tracking-wider">
                  Or Image Web URL (Optional)
                </label>
                <input
                  id="media-url-input"
                  type="text"
                  bind:value={uploadMediaUrl}
                  placeholder="https://example.com/backdrop.jpg"
                  class="px-3 py-2 rounded-lg text-xs {effectiveTheme === 'light' ? 'bg-stone-50 border-stone-300 text-stone-900 focus:bg-white' : 'bg-neutral-900 border-neutral-700 text-neutral-100 focus:bg-neutral-850'} border focus:outline-hidden"
                />
              </div>

              <!-- Live 16:9 Preview if available -->
              {#if uploadMediaPreview || uploadMediaUrl}
                <div class="flex flex-col gap-1">
                  <span class="text-[10px] font-semibold {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-400'} uppercase tracking-wider">
                    Thumbnail Preview
                  </span>
                  <div
                    class="w-full aspect-video rounded-lg border border-neutral-700 overflow-hidden"
                    style="background: url('{uploadMediaPreview || uploadMediaUrl}') center/cover no-repeat;"
                  ></div>
                </div>
              {/if}

              <!-- Modal Action Buttons -->
              <div class="flex items-center justify-end gap-2 pt-2 border-t {effectiveTheme === 'light' ? 'border-stone-100' : 'border-neutral-800'}">
                <button
                  onclick={() => showUploadModal = false}
                  class="px-3 py-1.5 rounded text-xs {effectiveTheme === 'light' ? 'text-stone-600 hover:bg-stone-100' : 'text-neutral-400 hover:bg-neutral-800'} transition-colors"
                >
                  Cancel
                </button>
                <button
                  onclick={saveCustomMedia}
                  class="px-4 py-1.5 rounded-lg bg-rose-600 hover:bg-rose-500 text-white text-xs font-semibold transition-colors flex items-center gap-1.5 shadow-xs"
                >
                  <Check class="w-3.5 h-3.5" />
                  <span>Save to Media Library</span>
                </button>
              </div>
            </div>
          </div>
        {/if}
      {:else if activeDockView === 'settings'}
        <!-- VS CODE STYLE SETTINGS & PREFERENCES VIEW -->
        <div class="flex flex-col h-full overflow-hidden {effectiveTheme === 'light' ? 'bg-[#f7f5f0]' : 'bg-[#09090c]'}">
          <!-- Settings Top Header -->
          <div class="p-4 border-b {effectiveTheme === 'light' ? 'border-stone-200 bg-white' : 'border-neutral-800/80 bg-[#0c0c10]'} flex items-center justify-between gap-3 shrink-0">
            <div class="flex items-center gap-2.5">
              <div class="p-1.5 rounded-lg {effectiveTheme === 'light' ? 'bg-indigo-50 border-indigo-200 text-indigo-600' : 'bg-indigo-950/40 border-indigo-800/60 text-indigo-400'} border">
                <Settings class="w-4 h-4" />
              </div>
              <div>
                <h2 class="text-xs font-bold {effectiveTheme === 'light' ? 'text-stone-900' : 'text-neutral-100'} uppercase tracking-wider">Preferences & Hardware</h2>
                <p class="text-[10px] {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'}">Configure typography, system fonts, speech engine, and audio hardware</p>
              </div>
            </div>
            <button
              onclick={() => activeDockView = 'scripture'}
              class="px-2.5 py-1.5 rounded {effectiveTheme === 'light' ? 'bg-stone-100 border-stone-200 hover:bg-stone-200 text-stone-700' : 'bg-neutral-900 border-neutral-800 hover:bg-neutral-800 text-neutral-300'} border text-xs font-medium transition-colors flex items-center gap-1.5 shadow-xs"
            >
              <span>← Back to Console</span>
            </button>
          </div>

          <!-- Settings Content (Scrollable) -->
          <div class="flex-1 overflow-y-auto p-5 flex flex-col gap-5 text-xs">
            <!-- Section 0: Interface Appearance -->
            <div class="p-4 rounded-xl {effectiveTheme === 'light' ? 'bg-white border-stone-200 shadow-xs' : 'bg-neutral-900/50 border-neutral-800/80'} border flex flex-col gap-3">
              <div class="flex items-center justify-between border-b {effectiveTheme === 'light' ? 'border-stone-100' : 'border-neutral-800'} pb-2">
                <span class="font-semibold {effectiveTheme === 'light' ? 'text-stone-800' : 'text-neutral-200'} flex items-center gap-2">
                  <Palette class="w-4 h-4 text-amber-500" />
                  <span>Interface Appearance</span>
                </span>
                <span class="text-[11px] {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-400'}">Desktop Theme</span>
              </div>

              <div class="grid grid-cols-3 gap-2.5 pt-1">
                <!-- Dark Mode Option -->
                <button
                  onclick={() => setAppTheme('dark')}
                  class="p-3 rounded-lg border text-left flex flex-col gap-2 transition-all {appTheme === 'dark' ? (effectiveTheme === 'light' ? 'bg-amber-50/80 border-amber-500 ring-1 ring-amber-400/40 text-stone-900 shadow-xs' : 'bg-neutral-800 border-amber-400/80 ring-1 ring-amber-400/50 font-medium') : (effectiveTheme === 'light' ? 'bg-stone-50/80 border-stone-200 hover:bg-stone-100 hover:border-stone-300' : 'bg-neutral-950/80 border-neutral-800 hover:border-neutral-700')}"
                >
                  <div class="flex items-center justify-between">
                    <Moon class="w-4 h-4 {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-200'}" />
                    {#if appTheme === 'dark'}
                      <Check class="w-3.5 h-3.5 text-amber-500" />
                    {/if}
                  </div>
                  <div>
                    <p class="text-xs font-semibold {effectiveTheme === 'light' ? 'text-stone-900' : 'text-neutral-200'}">Dark Mode</p>
                    <p class="text-[10px] {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'}">Studio obsidian</p>
                  </div>
                </button>

                <!-- Light Mode Option -->
                <button
                  onclick={() => setAppTheme('light')}
                  class="p-3 rounded-lg border text-left flex flex-col gap-2 transition-all {appTheme === 'light' ? (effectiveTheme === 'light' ? 'bg-amber-50/80 border-amber-500 ring-1 ring-amber-400/40 text-stone-900 shadow-xs' : 'bg-neutral-800 border-amber-400/80 ring-1 ring-amber-400/50 font-medium') : (effectiveTheme === 'light' ? 'bg-stone-50/80 border-stone-200 hover:bg-stone-100 hover:border-stone-300' : 'bg-neutral-950/80 border-neutral-800 hover:border-neutral-700')}"
                >
                  <div class="flex items-center justify-between">
                    <Sun class="w-4 h-4 text-amber-500" />
                    {#if appTheme === 'light'}
                      <Check class="w-3.5 h-3.5 text-amber-500" />
                    {/if}
                  </div>
                  <div>
                    <p class="text-xs font-semibold {effectiveTheme === 'light' ? 'text-stone-900' : 'text-neutral-200'}">Light Mode</p>
                    <p class="text-[10px] {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'}">Warm linen & stone</p>
                  </div>
                </button>

                <!-- Match System Option -->
                <button
                  onclick={() => setAppTheme('system')}
                  class="p-3 rounded-lg border text-left flex flex-col gap-2 transition-all {appTheme === 'system' ? (effectiveTheme === 'light' ? 'bg-amber-50/80 border-amber-500 ring-1 ring-amber-400/40 text-stone-900 shadow-xs' : 'bg-neutral-800 border-amber-400/80 ring-1 ring-amber-400/50 font-medium') : (effectiveTheme === 'light' ? 'bg-stone-50/80 border-stone-200 hover:bg-stone-100 hover:border-stone-300' : 'bg-neutral-950/80 border-neutral-800 hover:border-neutral-700')}"
                >
                  <div class="flex items-center justify-between">
                    <Monitor class="w-4 h-4 text-blue-500" />
                    {#if appTheme === 'system'}
                      <Check class="w-3.5 h-3.5 text-amber-500" />
                    {/if}
                  </div>
                  <div>
                    <p class="text-xs font-semibold {effectiveTheme === 'light' ? 'text-stone-900' : 'text-neutral-200'}">Match System</p>
                    <p class="text-[10px] {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'}">Sync with macOS</p>
                  </div>
                </button>
              </div>
            </div>

            <!-- Section 1: Scripture & Presentation Typography -->
            <div class="p-4 rounded-xl {effectiveTheme === 'light' ? 'bg-white border-stone-200 shadow-xs' : 'bg-neutral-900/50 border-neutral-800/80'} border flex flex-col gap-4">
              <div class="flex items-center justify-between border-b {effectiveTheme === 'light' ? 'border-stone-100' : 'border-neutral-800'} pb-2.5">
                <span class="font-semibold {effectiveTheme === 'light' ? 'text-stone-800' : 'text-neutral-200'} flex items-center gap-2">
                  <Type class="w-4 h-4 text-amber-500" />
                  <span>Scripture & Projection Typography</span>
                </span>
                <span class="text-[10px] font-mono {effectiveTheme === 'light' ? 'text-stone-600 bg-stone-100 border-stone-200' : 'text-neutral-400 bg-neutral-950 border-neutral-800'} px-2 py-0.5 rounded border">
                  Font: {projectorFontFamily} • Size: {projectorFontSize === 'auto' ? 'Auto-Fit' : `${projectorFontSize}px`}
                </span>
              </div>

              <!-- Font Family: System Fonts Picker -->
              <div class="flex flex-col gap-2">
                <div class="flex items-center justify-between">
                  <span class="text-[11px] font-semibold {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-300'}">System Font Family</span>
                  <span class="text-[10px] {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'} font-mono">{systemFonts.length} system fonts detected</span>
                </div>

                <!-- Search Input for System Fonts -->
                <div class="relative flex items-center">
                  <Search class="w-3.5 h-3.5 absolute left-3 {effectiveTheme === 'light' ? 'text-stone-400' : 'text-neutral-500'}" />
                  <input
                    type="text"
                    bind:value={fontSearchQuery}
                    placeholder="Search installed system fonts (e.g. Arial, Georgia, Helvetica, Baskerville)..."
                    class="w-full {effectiveTheme === 'light' ? 'bg-stone-50 border-stone-200 text-stone-900 placeholder-stone-400 focus:border-amber-500' : 'bg-neutral-950 border-neutral-800 text-neutral-200 placeholder-neutral-600 focus:border-amber-600/70'} rounded pl-9 pr-3 py-1.5 text-xs border focus:outline-none"
                  />
                  {#if fontSearchQuery}
                    <button
                      onclick={() => fontSearchQuery = ''}
                      class="absolute right-2.5 {effectiveTheme === 'light' ? 'text-stone-400 hover:text-stone-600' : 'text-neutral-500 hover:text-neutral-300'}"
                    >
                      <X class="w-3 h-3" />
                    </button>
                  {/if}
                </div>

                <!-- Popular Quick Font Chips -->
                <div class="flex items-center gap-1.5 flex-wrap">
                  <span class="text-[10px] uppercase font-mono {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'} mr-1">Quick:</span>
                  {#each ['Georgia', 'Arial', 'Baskerville', 'Helvetica', 'Charter', 'SF Mono', 'Times New Roman'] as qf}
                    <button
                      onclick={() => setTypography(qf, projectorFontSize)}
                      class="px-2 py-0.5 rounded text-[11px] border transition-colors {projectorFontFamily.toLowerCase() === qf.toLowerCase() ? (effectiveTheme === 'light' ? 'bg-amber-100 text-amber-900 border-amber-400 font-bold shadow-xs' : 'bg-amber-500/20 text-amber-300 border-amber-600/80 font-bold') : (effectiveTheme === 'light' ? 'bg-stone-100/80 text-stone-700 border-stone-200 hover:bg-stone-200/70 hover:text-stone-900' : 'bg-neutral-950 text-neutral-400 border-neutral-800 hover:text-neutral-200 hover:border-neutral-700')}"
                      style="font-family: '{qf}', sans-serif;"
                    >
                      {qf}
                    </button>
                  {/each}
                </div>

                <!-- Filtered System Fonts Scrollbox -->
                <div class="max-h-36 overflow-y-auto border {effectiveTheme === 'light' ? 'border-stone-200 bg-stone-50/80' : 'border-neutral-800 bg-neutral-950/60'} rounded-lg p-1.5 grid grid-cols-2 gap-1">
                  {#each filteredSystemFonts as sFont}
                    <button
                      onclick={() => setTypography(sFont, projectorFontSize)}
                      class="text-left px-2.5 py-1.5 rounded text-xs truncate transition-colors flex items-center justify-between group {projectorFontFamily.toLowerCase() === sFont.toLowerCase() ? (effectiveTheme === 'light' ? 'bg-amber-100 text-amber-900 border border-amber-400 font-semibold' : 'bg-amber-500/20 text-amber-300 border border-amber-600/60 font-semibold') : (effectiveTheme === 'light' ? 'text-stone-700 hover:bg-stone-200/60 hover:text-stone-900' : 'text-neutral-400 hover:bg-neutral-900 hover:text-neutral-100')}"
                    >
                      <span class="truncate" style="font-family: '{sFont}', sans-serif;">{sFont}</span>
                      {#if projectorFontFamily.toLowerCase() === sFont.toLowerCase()}
                        <Check class="w-3 h-3 text-amber-500 shrink-0 ml-1" />
                      {/if}
                    </button>
                  {/each}
                </div>
              </div>

              <!-- Scripture Reference Position -->
              <div class="flex flex-col gap-2 pt-2 border-t {effectiveTheme === 'light' ? 'border-stone-100' : 'border-neutral-800/60'}">
                <div class="flex items-center justify-between">
                  <div>
                    <span class="text-[11px] font-semibold {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-300'}">Scripture Reference Position</span>
                    <p class="text-[10px] {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'}">Choose where the scripture reference title appears on the slide</p>
                  </div>
                  <span class="text-[10px] font-mono {effectiveTheme === 'light' ? 'text-amber-800 bg-amber-50 border-amber-200' : 'text-amber-300 bg-amber-950/50 border-amber-800/60'} px-2 py-0.5 rounded border capitalize">
                    {projectorRefPosition.replace('-', ' ')}
                  </span>
                </div>

                <!-- Position Visual Grid -->
                <div class="grid grid-cols-4 sm:grid-cols-7 gap-1.5 pt-1">
                  {#each [
                    { id: 'top-left', label: 'Top Left' },
                    { id: 'top-center', label: 'Top Center' },
                    { id: 'top-right', label: 'Top Right' },
                    { id: 'bottom-left', label: 'Bottom Left' },
                    { id: 'bottom-center', label: 'Bottom Center' },
                    { id: 'bottom-right', label: 'Bottom Right' },
                    { id: 'inline', label: 'Inline End' }
                  ] as pos}
                    <button
                      onclick={() => setLayout({ refPosition: pos.id })}
                      class="p-2 rounded-lg border text-center flex flex-col items-center justify-center gap-1.5 transition-all {projectorRefPosition === pos.id ? (effectiveTheme === 'light' ? 'bg-amber-100/90 text-amber-950 border-amber-500 font-bold shadow-xs ring-1 ring-amber-400/40' : 'bg-amber-500/20 text-amber-300 border-amber-500 font-bold ring-1 ring-amber-500/50') : (effectiveTheme === 'light' ? 'bg-stone-50 text-stone-700 border-stone-200 hover:bg-stone-100 hover:text-stone-900' : 'bg-neutral-950 text-neutral-400 border-neutral-800 hover:text-neutral-200 hover:border-neutral-700')}"
                    >
                      <!-- Mini Screen Representation -->
                      <div class="w-8 h-5 rounded border {projectorRefPosition === pos.id ? 'border-amber-400/80 bg-amber-500/10' : (effectiveTheme === 'light' ? 'border-stone-300 bg-white' : 'border-neutral-700 bg-neutral-900')} relative flex p-0.5 overflow-hidden">
                        {#if pos.id === 'top-left'}
                          <div class="w-2.5 h-1 rounded-[1px] bg-amber-500 absolute top-0.5 left-0.5"></div>
                          <div class="w-5 h-1 rounded-[1px] bg-neutral-400/40 absolute top-2.5 left-1.5"></div>
                        {:else if pos.id === 'top-center'}
                          <div class="w-3 h-1 rounded-[1px] bg-amber-500 absolute top-0.5 left-1/2 -translate-x-1/2"></div>
                          <div class="w-5 h-1 rounded-[1px] bg-neutral-400/40 absolute top-2.5 left-1.5"></div>
                        {:else if pos.id === 'top-right'}
                          <div class="w-2.5 h-1 rounded-[1px] bg-amber-500 absolute top-0.5 right-0.5"></div>
                          <div class="w-5 h-1 rounded-[1px] bg-neutral-400/40 absolute top-2.5 left-1.5"></div>
                        {:else if pos.id === 'bottom-left'}
                          <div class="w-5 h-1 rounded-[1px] bg-neutral-400/40 absolute top-1.5 left-1.5"></div>
                          <div class="w-2.5 h-1 rounded-[1px] bg-amber-500 absolute bottom-0.5 left-0.5"></div>
                        {:else if pos.id === 'bottom-center'}
                          <div class="w-5 h-1 rounded-[1px] bg-neutral-400/40 absolute top-1.5 left-1.5"></div>
                          <div class="w-3 h-1 rounded-[1px] bg-amber-500 absolute bottom-0.5 left-1/2 -translate-x-1/2"></div>
                        {:else if pos.id === 'bottom-right'}
                          <div class="w-5 h-1 rounded-[1px] bg-neutral-400/40 absolute top-1.5 left-1.5"></div>
                          <div class="w-2.5 h-1 rounded-[1px] bg-amber-500 absolute bottom-0.5 right-0.5"></div>
                        {:else if pos.id === 'inline'}
                          <div class="w-4 h-1 rounded-[1px] bg-neutral-400/40 absolute top-2 left-1"></div>
                          <div class="w-2 h-1 rounded-[1px] bg-amber-500 absolute top-2 right-1"></div>
                        {/if}
                      </div>
                      <span class="text-[10px] leading-tight truncate">{pos.label}</span>
                    </button>
                  {/each}
                </div>
              </div>

              <!-- Scripture Verse Coverage on Screen -->
              <div class="flex flex-col gap-2 pt-2 border-t {effectiveTheme === 'light' ? 'border-stone-100' : 'border-neutral-800/60'}">
                <div class="flex items-center justify-between">
                  <div>
                    <span class="text-[11px] font-semibold {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-300'}">Screen Verse Coverage (Safe Margin)</span>
                    <p class="text-[10px] {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'}">Controls maximum screen bounding box occupied by the verse</p>
                  </div>
                  <span class="text-[10px] font-mono {effectiveTheme === 'light' ? 'text-stone-700 bg-stone-100 border-stone-200' : 'text-neutral-300 bg-neutral-950 border-neutral-800'} px-2 py-0.5 rounded border font-bold">
                    {projectorVerseCoverage}% Width
                  </span>
                </div>

                <div class="flex items-center gap-3 p-3 rounded-lg {effectiveTheme === 'light' ? 'bg-stone-100/70 border-stone-200' : 'bg-neutral-950/60 border-neutral-800'} border">
                  <!-- Presets -->
                  <div class="flex items-center gap-1.5">
                    {#each [
                      { val: 60, label: '60% Compact' },
                      { val: 75, label: '75% Balanced' },
                      { val: 85, label: '85% Standard' },
                      { val: 95, label: '95% Full' }
                    ] as cov}
                      <button
                        onclick={() => setLayout({ verseCoverage: cov.val })}
                        class="px-2 py-1 rounded text-[10px] font-mono font-medium transition-colors border {projectorVerseCoverage === cov.val ? (effectiveTheme === 'light' ? 'bg-amber-100 text-amber-900 border-amber-400 font-bold shadow-xs' : 'bg-amber-500/20 text-amber-300 border-amber-600/80 font-bold') : (effectiveTheme === 'light' ? 'bg-white text-stone-700 border-stone-200 hover:bg-stone-100' : 'bg-neutral-900 text-neutral-400 border-neutral-800 hover:border-neutral-700')}"
                      >
                        {cov.label}
                      </button>
                    {/each}
                  </div>

                  <!-- Continuous Slider -->
                  <input
                    type="range"
                    min="50"
                    max="95"
                    step="5"
                    value={projectorVerseCoverage}
                    oninput={(e) => setLayout({ verseCoverage: Number((e.target as HTMLInputElement).value) })}
                    class="flex-1 accent-amber-500 cursor-pointer h-1.5 {effectiveTheme === 'light' ? 'bg-stone-200' : 'bg-neutral-800'} rounded-lg"
                  />
                </div>
              </div>

              <!-- Font Size & Auto-Fit Engine (FreeShow Style) -->
              <div class="flex flex-col gap-2.5 pt-2 border-t {effectiveTheme === 'light' ? 'border-stone-100' : 'border-neutral-800/60'}">
                <div class="flex items-center justify-between">
                  <div>
                    <span class="text-[11px] font-semibold {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-300'}">Auto-Fit Engine (FreeShow Style)</span>
                    <p class="text-[10px] {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'}">Automatically scale typography to optimize screen readability</p>
                  </div>
                  <!-- Auto-Fit Master Switch -->
                  <button
                    onclick={toggleAutoFit}
                    class="flex items-center gap-1.5 px-2.5 py-1 rounded text-xs font-mono font-semibold transition-colors border {projectorAutoFit ? (effectiveTheme === 'light' ? 'bg-emerald-50 text-emerald-800 border-emerald-300 shadow-xs' : 'bg-emerald-950/60 text-emerald-300 border-emerald-700/80') : (effectiveTheme === 'light' ? 'bg-stone-100 text-stone-600 border-stone-200 hover:border-stone-300' : 'bg-neutral-950 text-neutral-400 border-neutral-800 hover:border-neutral-700')}"
                  >
                    <span class="w-1.5 h-1.5 rounded-full {projectorAutoFit ? 'bg-emerald-500' : 'bg-neutral-500'}"></span>
                    <span>Auto-Fit {projectorAutoFit ? 'ON' : 'OFF'}</span>
                  </button>
                </div>

                {#if projectorAutoFit}
                  <div class="p-3.5 rounded-lg {effectiveTheme === 'light' ? 'bg-stone-100/70 border-stone-200' : 'bg-neutral-950/60 border-neutral-800'} border flex flex-col gap-3">
                    <!-- Auto-Grow & Auto-Shrink Switches -->
                    <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                      <!-- Auto-Grow Switch -->
                      <button
                        onclick={() => setLayout({ autoGrow: !projectorAutoGrow })}
                        class="p-2.5 rounded-lg border text-left flex items-start justify-between gap-2 transition-colors {projectorAutoGrow ? (effectiveTheme === 'light' ? 'bg-white border-amber-300 shadow-xs' : 'bg-neutral-900 border-amber-600/60') : (effectiveTheme === 'light' ? 'bg-stone-50/60 border-stone-200 text-stone-500' : 'bg-neutral-950 border-neutral-800 text-neutral-500')}"
                      >
                        <div>
                          <div class="flex items-center gap-1.5">
                            <span class="font-semibold text-xs {projectorAutoGrow ? (effectiveTheme === 'light' ? 'text-stone-900 font-bold' : 'text-neutral-100 font-bold') : ''}">Auto-Grow</span>
                            <span class="text-[9px] px-1.5 py-0.2 rounded font-mono uppercase {projectorAutoGrow ? 'bg-amber-100 text-amber-800 border border-amber-300' : 'bg-stone-200 text-stone-600'}">
                              {projectorAutoGrow ? 'Active' : 'Off'}
                            </span>
                          </div>
                          <p class="text-[10px] {effectiveTheme === 'light' ? 'text-stone-600' : 'text-neutral-400'} mt-0.5">Short verses scale up (up to {projectorMaxFontSize}px) to fill empty space.</p>
                        </div>
                        <div class="w-8 h-4 rounded-full transition-colors flex items-center p-0.5 shrink-0 mt-0.5 {projectorAutoGrow ? 'bg-amber-500 justify-end' : (effectiveTheme === 'light' ? 'bg-stone-300 justify-start' : 'bg-neutral-700 justify-start')}">
                          <div class="w-3 h-3 rounded-full bg-white shadow-xs"></div>
                        </div>
                      </button>

                      <!-- Auto-Shrink Switch -->
                      <button
                        onclick={() => setLayout({ autoShrink: !projectorAutoShrink })}
                        class="p-2.5 rounded-lg border text-left flex items-start justify-between gap-2 transition-colors {projectorAutoShrink ? (effectiveTheme === 'light' ? 'bg-white border-amber-300 shadow-xs' : 'bg-neutral-900 border-amber-600/60') : (effectiveTheme === 'light' ? 'bg-stone-50/60 border-stone-200 text-stone-500' : 'bg-neutral-950 border-neutral-800 text-neutral-500')}"
                      >
                        <div>
                          <div class="flex items-center gap-1.5">
                            <span class="font-semibold text-xs {projectorAutoShrink ? (effectiveTheme === 'light' ? 'text-stone-900 font-bold' : 'text-neutral-100 font-bold') : ''}">Auto-Shrink</span>
                            <span class="text-[9px] px-1.5 py-0.2 rounded font-mono uppercase {projectorAutoShrink ? 'bg-amber-100 text-amber-800 border border-amber-300' : 'bg-stone-200 text-stone-600'}">
                              {projectorAutoShrink ? 'Active' : 'Off'}
                            </span>
                          </div>
                          <p class="text-[10px] {effectiveTheme === 'light' ? 'text-stone-600' : 'text-neutral-400'} mt-0.5">Long multi-verse passages shrink (down to {projectorMinFontSize}px) to prevent clipping.</p>
                        </div>
                        <div class="w-8 h-4 rounded-full transition-colors flex items-center p-0.5 shrink-0 mt-0.5 {projectorAutoShrink ? 'bg-amber-500 justify-end' : (effectiveTheme === 'light' ? 'bg-stone-300 justify-start' : 'bg-neutral-700 justify-start')}">
                          <div class="w-3 h-3 rounded-full bg-white shadow-xs"></div>
                        </div>
                      </button>
                    </div>

                    <!-- Min & Max Bounds Sliders -->
                    <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-2 border-t {effectiveTheme === 'light' ? 'border-stone-200/80' : 'border-neutral-800/80'}">
                      <!-- Min Limit (Floor) -->
                      <div class="flex flex-col gap-1">
                        <div class="flex items-center justify-between text-[10px]">
                          <span class="{effectiveTheme === 'light' ? 'text-stone-600' : 'text-neutral-400'} font-medium">Auto-Shrink Limit (Min Font Size)</span>
                          <span class="font-mono font-bold {effectiveTheme === 'light' ? 'text-stone-900' : 'text-neutral-200'}">{projectorMinFontSize}px</span>
                        </div>
                        <div class="flex items-center gap-2">
                          <button
                            onclick={() => setLayout({ minFontSize: Math.max(18, projectorMinFontSize - 2) })}
                            class="px-2 py-0.5 rounded border {effectiveTheme === 'light' ? 'bg-white border-stone-200 text-stone-700 hover:bg-stone-100' : 'bg-neutral-900 border-neutral-800 text-neutral-300 hover:bg-neutral-800'}"
                          >-</button>
                          <input
                            type="range"
                            min="18"
                            max="40"
                            step="2"
                            value={projectorMinFontSize}
                            oninput={(e) => setLayout({ minFontSize: Number((e.target as HTMLInputElement).value) })}
                            class="flex-1 accent-amber-500 cursor-pointer h-1.5 {effectiveTheme === 'light' ? 'bg-stone-200' : 'bg-neutral-800'} rounded-lg"
                          />
                          <button
                            onclick={() => setLayout({ minFontSize: Math.min(40, projectorMinFontSize + 2) })}
                            class="px-2 py-0.5 rounded border {effectiveTheme === 'light' ? 'bg-white border-stone-200 text-stone-700 hover:bg-stone-100' : 'bg-neutral-900 border-neutral-800 text-neutral-300 hover:bg-neutral-800'}"
                          >+</button>
                        </div>
                      </div>

                      <!-- Max Limit (Ceiling) -->
                      <div class="flex flex-col gap-1">
                        <div class="flex items-center justify-between text-[10px]">
                          <span class="{effectiveTheme === 'light' ? 'text-stone-600' : 'text-neutral-400'} font-medium">Auto-Grow Limit (Max Font Size)</span>
                          <span class="font-mono font-bold {effectiveTheme === 'light' ? 'text-stone-900' : 'text-neutral-200'}">{projectorMaxFontSize}px</span>
                        </div>
                        <div class="flex items-center gap-2">
                          <button
                            onclick={() => setLayout({ maxFontSize: Math.max(56, projectorMaxFontSize - 4) })}
                            class="px-2 py-0.5 rounded border {effectiveTheme === 'light' ? 'bg-white border-stone-200 text-stone-700 hover:bg-stone-100' : 'bg-neutral-900 border-neutral-800 text-neutral-300 hover:bg-neutral-800'}"
                          >-</button>
                          <input
                            type="range"
                            min="56"
                            max="120"
                            step="4"
                            value={projectorMaxFontSize}
                            oninput={(e) => setLayout({ maxFontSize: Number((e.target as HTMLInputElement).value) })}
                            class="flex-1 accent-amber-500 cursor-pointer h-1.5 {effectiveTheme === 'light' ? 'bg-stone-200' : 'bg-neutral-800'} rounded-lg"
                          />
                          <button
                            onclick={() => setLayout({ maxFontSize: Math.min(120, projectorMaxFontSize + 4) })}
                            class="px-2 py-0.5 rounded border {effectiveTheme === 'light' ? 'bg-white border-stone-200 text-stone-700 hover:bg-stone-100' : 'bg-neutral-900 border-neutral-800 text-neutral-300 hover:bg-neutral-800'}"
                          >+</button>
                        </div>
                      </div>
                    </div>
                  </div>
                {:else}
                  <!-- Exact Manual Point Size Controls -->
                  <div class="flex items-center gap-3 p-3 rounded-lg {effectiveTheme === 'light' ? 'bg-stone-100/70 border-stone-200' : 'bg-neutral-950/60 border-neutral-800'} border">
                    <div class="flex items-center rounded border {effectiveTheme === 'light' ? 'border-stone-200 bg-white' : 'border-neutral-800 bg-neutral-900'} p-0.5">
                      <button
                        onclick={() => adjustFontSize(-4)}
                        class="px-2.5 py-1 rounded {effectiveTheme === 'light' ? 'text-stone-600 hover:text-stone-900 hover:bg-stone-100' : 'text-neutral-400 hover:text-neutral-100 hover:bg-neutral-800'} transition-colors"
                      >
                        -
                      </button>
                      <input
                        type="number"
                        min="16"
                        max="160"
                        step="2"
                        value={isNumericFontSize ? Number(projectorFontSize) : 48}
                        onchange={(e) => setTypography(projectorFontFamily, (e.target as HTMLInputElement).value)}
                        class="w-14 bg-transparent text-center text-xs font-mono font-bold {effectiveTheme === 'light' ? 'text-stone-900' : 'text-neutral-100'} focus:outline-none"
                      />
                      <span class="text-[10px] font-mono {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'} pr-1.5">px</span>
                      <button
                        onclick={() => adjustFontSize(4)}
                        class="px-2.5 py-1 rounded {effectiveTheme === 'light' ? 'text-stone-600 hover:text-stone-900 hover:bg-stone-100' : 'text-neutral-400 hover:text-neutral-100 hover:bg-neutral-800'} transition-colors"
                      >
                        +
                      </button>
                    </div>

                    <input
                      type="range"
                      min="20"
                      max="120"
                      step="2"
                      value={isNumericFontSize ? Number(projectorFontSize) : 48}
                      oninput={(e) => setTypography(projectorFontFamily, (e.target as HTMLInputElement).value)}
                      class="flex-1 accent-amber-500 cursor-pointer h-1.5 {effectiveTheme === 'light' ? 'bg-stone-200' : 'bg-neutral-800'} rounded-lg"
                    />

                    <div class="flex items-center gap-1">
                      {#each ['32', '40', '48', '56', '64', '76'] as sz}
                        <button
                          onclick={() => setTypography(projectorFontFamily, sz)}
                          class="px-1.5 py-0.5 rounded text-[10px] font-mono transition-colors border {projectorFontSize === sz ? (effectiveTheme === 'light' ? 'bg-stone-900 text-stone-50 border-stone-900 font-bold' : 'bg-neutral-200 text-neutral-950 border-neutral-200 font-bold') : (effectiveTheme === 'light' ? 'bg-white text-stone-700 border-stone-200 hover:bg-stone-100' : 'bg-neutral-900 text-neutral-400 border-neutral-800 hover:border-neutral-700')}"
                        >
                          {sz}
                        </button>
                      {/each}
                    </div>
                  </div>
                {/if}
              </div>

              <!-- Translation & Projection Assist -->
              <div class="grid grid-cols-2 gap-3 pt-2 border-t {effectiveTheme === 'light' ? 'border-stone-100' : 'border-neutral-800/60'}">
                <div class="flex flex-col gap-1.5">
                  <span class="text-[11px] font-semibold {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-300'}">Default Bible Translation</span>
                  <div class="flex items-center rounded border {effectiveTheme === 'light' ? 'border-stone-200 bg-stone-100' : 'border-neutral-800 bg-neutral-950'} p-0.5 text-xs">
                    <button
                      onclick={() => changeTranslation('KJV')}
                      class="flex-1 py-1 rounded font-medium transition-colors {activeTranslation === 'KJV' ? (effectiveTheme === 'light' ? 'bg-white text-stone-900 font-semibold shadow-xs' : 'bg-neutral-800 text-neutral-100') : (effectiveTheme === 'light' ? 'text-stone-500 hover:text-stone-800' : 'text-neutral-500 hover:text-neutral-300')}"
                    >
                      KJV (King James)
                    </button>
                    <button
                      onclick={() => changeTranslation('WEB')}
                      class="flex-1 py-1 rounded font-medium transition-colors {activeTranslation === 'WEB' ? (effectiveTheme === 'light' ? 'bg-white text-stone-900 font-semibold shadow-xs' : 'bg-neutral-800 text-neutral-100') : (effectiveTheme === 'light' ? 'text-stone-500 hover:text-stone-800' : 'text-neutral-500 hover:text-neutral-300')}"
                    >
                      WEB (World English)
                    </button>
                  </div>
                </div>

                <div class="flex flex-col gap-1.5">
                  <span class="text-[11px] font-semibold {effectiveTheme === 'light' ? 'text-stone-700' : 'text-neutral-300'}">Projection Assist Mode</span>
                  <button
                    onclick={toggleAutoProject}
                    class="py-1.5 px-3 rounded border text-xs font-medium transition-colors {autoProject ? (effectiveTheme === 'light' ? 'bg-stone-900 text-stone-50 border-stone-900 font-semibold' : 'bg-neutral-200 text-neutral-950 border-neutral-200 font-semibold') : (effectiveTheme === 'light' ? 'bg-stone-100 text-stone-700 border-stone-200 hover:bg-stone-200/70' : 'bg-neutral-950 text-neutral-300 border-neutral-800 hover:border-neutral-700')}"
                  >
                    {autoProject ? 'Auto-Pilot (Instant Push)' : 'Co-Pilot (3.0s Countdown)'}
                  </button>
                </div>
              </div>
            </div>

            <!-- Section 2: Audio Input Hardware -->
            <div class="p-4 rounded-xl {effectiveTheme === 'light' ? 'bg-white border-stone-200 shadow-xs' : 'bg-neutral-900/50 border-neutral-800/80'} border flex flex-col gap-3">
              <div class="flex items-center justify-between border-b {effectiveTheme === 'light' ? 'border-stone-100' : 'border-neutral-800'} pb-2">
                <span class="font-semibold {effectiveTheme === 'light' ? 'text-stone-800' : 'text-neutral-200'} flex items-center gap-2">
                  <Mic class="w-4 h-4 text-emerald-500" />
                  <span>Audio Input Hardware</span>
                </span>
                <button
                  onclick={refreshAudioDevices}
                  class="text-[11px] {effectiveTheme === 'light' ? 'text-stone-500 hover:text-stone-800' : 'text-neutral-400 hover:text-neutral-200'} underline"
                >
                  Refresh Devices
                </button>
              </div>

              <div class="flex flex-col gap-2">
                {#if devices.length > 0}
                  <select
                    bind:value={selectedDevice}
                    class="{effectiveTheme === 'light' ? 'bg-stone-50 border-stone-200 text-stone-900 focus:border-amber-500' : 'bg-neutral-950 border-neutral-800 text-neutral-200 focus:border-neutral-600'} border text-xs rounded px-3 py-2 focus:outline-none w-full"
                  >
                    {#each devices as dev}
                      <option value={dev.name}>{dev.name} {dev.is_default ? '(System Default)' : ''}</option>
                    {/each}
                  </select>
                {:else}
                  <p class="text-neutral-500 italic">No audio input devices detected.</p>
                {/if}

                <div class="flex items-center justify-between text-[11px] {effectiveTheme === 'light' ? 'text-stone-600' : 'text-neutral-400'} pt-1">
                  <span>Microphone Signal VU:</span>
                  <div class="w-48 h-2.5 {effectiveTheme === 'light' ? 'bg-stone-200 border-stone-300' : 'bg-neutral-950 border-neutral-800'} rounded-full overflow-hidden border flex items-center px-0.5">
                    <div
                      class="h-1.5 rounded-full transition-all duration-75 {audioLevel > 0.6 ? 'bg-amber-500' : 'bg-emerald-500'}"
                      style="width: {Math.max(4, Math.round(audioLevel * 100))}%"
                    ></div>
                  </div>
                </div>
              </div>
            </div>

            <!-- Section 3: Speech Recognition Engine -->
            <div class="p-4 rounded-xl {effectiveTheme === 'light' ? 'bg-white border-stone-200 shadow-xs' : 'bg-neutral-900/50 border-neutral-800/80'} border flex flex-col gap-3">
              <div class="flex items-center justify-between border-b {effectiveTheme === 'light' ? 'border-stone-100' : 'border-neutral-800'} pb-2">
                <span class="font-semibold {effectiveTheme === 'light' ? 'text-stone-800' : 'text-neutral-200'} flex items-center gap-2">
                  <Volume2 class="w-4 h-4 text-indigo-500" />
                  <span>Speech Recognition Engine</span>
                </span>
                <span class="px-2 py-0.5 rounded text-[10px] font-mono {modelReady ? (effectiveTheme === 'light' ? 'bg-emerald-50 text-emerald-800 border border-emerald-300' : 'bg-emerald-950/60 text-emerald-300 border border-emerald-800/70') : modelDownloading ? (effectiveTheme === 'light' ? 'bg-amber-50 text-amber-800 border border-amber-300' : 'bg-amber-950/60 text-amber-300 border border-amber-800/70') : (effectiveTheme === 'light' ? 'bg-stone-100 text-stone-600 border border-stone-200' : 'bg-neutral-900 text-neutral-400 border border-neutral-800')}">
                  {modelReady ? 'Ready (Offline Local)' : modelDownloading ? `Downloading ${downloadPercent}%` : 'Not Ready'}
                </span>
              </div>
              <p class="{effectiveTheme === 'light' ? 'text-stone-600' : 'text-neutral-400'} text-[11px] leading-relaxed">
                Active model: <code class="font-mono {effectiveTheme === 'light' ? 'text-stone-800 bg-stone-100 border-stone-200' : 'text-neutral-200 bg-neutral-950 border-neutral-800'} px-1.5 py-0.5 rounded border">ggml-base.en.bin</code>. Executes completely on-device with zero internet dependency and deep regional Ghanaian & West African phonetic matching.
              </p>
              {#if !modelReady && !modelDownloading}
                <button
                  onclick={startModelDownload}
                  class="self-start mt-1 px-3 py-1.5 rounded {effectiveTheme === 'light' ? 'bg-stone-100 hover:bg-stone-200 text-stone-800 border border-stone-200' : 'bg-neutral-800 hover:bg-neutral-700 text-neutral-200'} text-xs font-medium transition-colors flex items-center gap-1.5"
                >
                  <DownloadCloud class="w-3.5 h-3.5" />
                  Download Model File
                </button>
              {/if}
            </div>

            <!-- Section 4: Keyboard Shortcuts -->
            <div class="p-4 rounded-xl {effectiveTheme === 'light' ? 'bg-white border-stone-200 shadow-xs' : 'bg-neutral-900/50 border-neutral-800/80'} border flex flex-col gap-2.5">
              <span class="font-semibold {effectiveTheme === 'light' ? 'text-stone-800' : 'text-neutral-200'}">Operator Keyboard Shortcuts</span>
              <div class="grid grid-cols-2 gap-x-6 gap-y-2 text-[11px] {effectiveTheme === 'light' ? 'text-stone-600' : 'text-neutral-400'}">
                <div class="flex items-center justify-between">
                  <span>Push Scripture Live:</span>
                  <kbd class="px-2 py-0.5 rounded {effectiveTheme === 'light' ? 'bg-stone-100 border-stone-300 text-stone-800 shadow-xs' : 'bg-neutral-950 border-neutral-800 text-neutral-300'} font-mono border">Space</kbd>
                </div>
                <div class="flex items-center justify-between">
                  <span>Dismiss Intent / Return:</span>
                  <kbd class="px-2 py-0.5 rounded {effectiveTheme === 'light' ? 'bg-stone-100 border-stone-300 text-stone-800 shadow-xs' : 'bg-neutral-950 border-neutral-800 text-neutral-300'} font-mono border">Esc</kbd>
                </div>
                <div class="flex items-center justify-between">
                  <span>Clear Slide:</span>
                  <kbd class="px-2 py-0.5 rounded {effectiveTheme === 'light' ? 'bg-stone-100 border-stone-300 text-stone-800 shadow-xs' : 'bg-neutral-950 border-neutral-800 text-neutral-300'} font-mono border">C</kbd>
                </div>
                <div class="flex items-center justify-between">
                  <span>Blackout Screen:</span>
                  <kbd class="px-2 py-0.5 rounded {effectiveTheme === 'light' ? 'bg-stone-100 border-stone-300 text-stone-800 shadow-xs' : 'bg-neutral-950 border-neutral-800 text-neutral-300'} font-mono border">B</kbd>
                </div>
              </div>
            </div>
          </div>
        </div>
      {/if}
    </div>

    <!-- RIGHT PANEL: STAGE MONITOR & MANUAL SCRIPTURE SEARCH (5 Cols) -->
    <div class="col-span-5 flex flex-col h-full bg-[#0b0b0e] overflow-hidden">
      <!-- Live Stage Monitor Preview -->
      <div class="p-5 border-b border-neutral-800 flex flex-col gap-3 shrink-0">
        <div class="flex items-center justify-between">
          <span class="text-xs font-semibold tracking-wide uppercase text-neutral-400 flex items-center gap-1.5">
            <Monitor class="w-3.5 h-3.5 text-neutral-400" />
            Live Projector Monitor
          </span>
          <span class="text-[10px] font-mono text-neutral-500 uppercase flex items-center gap-1.5">
            <span class="w-1.5 h-1.5 rounded-full {projectorBlackout ? 'bg-neutral-600' : projectorClear ? 'bg-amber-400' : 'bg-emerald-400'}"></span>
            {projectorBlackout ? 'Blackout' : projectorClear ? 'Cleared' : 'Live Output'}
          </span>
        </div>

        <!-- Scaled Monitor Box (Exact 1:1 Mirror of Second Screen Output, No divider line) -->
        <div
          bind:clientWidth={monitorContainerWidth}
          class="aspect-video w-full rounded-lg border border-neutral-800 overflow-hidden relative flex flex-col transition-colors select-none"
          style="{projectorThemeBackgroundStyle}"
        >
          <!-- Atmospheric Ambient Glow Lighting in Live Monitor (Exact 1:1 Mirror of Second Screen) -->
          {#if !projectorBlackout && projectorTheme !== 'lower-third' && projectorTheme !== 'light' && projectorTheme !== 'minimal-black' && projectorTheme !== 'black'}
            <div class="absolute inset-0 pointer-events-none overflow-hidden">
              {#if projectorTheme === 'celestial'}
                <div class="absolute -top-[20%] -left-[10%] w-[60%] h-[60%] rounded-full bg-purple-600/15 blur-2xl"></div>
                <div class="absolute top-[30%] -right-[15%] w-[55%] h-[55%] rounded-full bg-fuchsia-600/15 blur-2xl"></div>
              {:else if projectorTheme === 'gold'}
                <div class="absolute -top-[20%] left-[20%] w-[60%] h-[60%] rounded-full bg-amber-600/10 blur-2xl"></div>
              {:else if projectorTheme === 'emerald'}
                <div class="absolute -top-[20%] -left-[10%] w-[60%] h-[60%] rounded-full bg-emerald-600/15 blur-2xl"></div>
                <div class="absolute top-[30%] -right-[15%] w-[55%] h-[55%] rounded-full bg-teal-600/15 blur-2xl"></div>
              {:else if projectorTheme === 'crimson'}
                <div class="absolute -top-[20%] -left-[10%] w-[60%] h-[60%] rounded-full bg-rose-600/15 blur-2xl"></div>
                <div class="absolute top-[30%] -right-[15%] w-[55%] h-[55%] rounded-full bg-red-700/15 blur-2xl"></div>
              {:else if projectorTheme === 'ocean'}
                <div class="absolute -top-[20%] -left-[10%] w-[60%] h-[60%] rounded-full bg-cyan-600/15 blur-2xl"></div>
                <div class="absolute top-[30%] -right-[15%] w-[55%] h-[55%] rounded-full bg-teal-600/15 blur-2xl"></div>
              {:else if projectorTheme === 'sunset'}
                <div class="absolute -top-[20%] -left-[10%] w-[60%] h-[60%] rounded-full bg-orange-600/15 blur-2xl"></div>
                <div class="absolute top-[30%] -right-[15%] w-[55%] h-[55%] rounded-full bg-amber-600/15 blur-2xl"></div>
              {:else if !projectorTheme.startsWith('url(') && !projectorTheme.startsWith('data:') && !projectorTheme.startsWith('http')}
                <!-- Atmospheric Blue/Teal (Screenshot Style) -->
                <div class="absolute -top-[20%] -left-[10%] w-[60%] h-[60%] rounded-full bg-cyan-600/15 blur-2xl"></div>
                <div class="absolute top-[30%] -right-[15%] w-[55%] h-[55%] rounded-full bg-blue-600/15 blur-2xl"></div>
                <div class="absolute -bottom-[20%] left-[20%] w-[50%] h-[50%] rounded-full bg-indigo-700/15 blur-2xl"></div>
              {/if}
            </div>
          {/if}

          {#if projectorBlackout}
            <div class="flex-1 flex items-center justify-center relative z-10">
              <span class="text-xs font-mono tracking-widest text-neutral-600 uppercase">[ Output Blacked Out ]</span>
            </div>
          {:else if projectorClear || !currentPassage}
            <div class="flex-1 flex items-center justify-center relative z-10">
              <span class="text-xs font-mono tracking-widest text-neutral-600 uppercase">[ Screen Cleared ]</span>
            </div>
          {:else}
            <!-- 1920x1080 Scaled Virtual Stage Canvas (Guarantees exact 1:1 structure and wrapping) -->
            <div
              class="projection-canvas w-[1920px] h-[1080px] origin-top-left absolute top-0 left-0 flex flex-col pointer-events-none transition-colors relative z-10"
              style="transform: scale({(monitorContainerWidth || 380) / 1920}); font-family: {projectorFontFamilyStyle};"
            >
              {#if projectorTheme === 'lower-third'}
                <!-- LOWER-THIRD BROADCAST OVERLAY -->
                <div class="mt-auto w-full p-16 pb-24 bg-gradient-to-t from-black/95 via-black/80 to-transparent flex flex-col items-center relative z-10">
                  <div class="w-full max-w-5xl rounded-2xl bg-neutral-950/95 border border-neutral-800/80 p-10 shadow-2xl flex flex-col gap-4 backdrop-blur-md">
                    <p
                      class="text-3xl text-neutral-100 font-bold leading-relaxed whitespace-pre-line text-center"
                      style="font-family: {projectorFontFamilyStyle}; font-size: {Math.min(dynamicFontSizePx, 36)}px; text-shadow: 0 2px 8px rgba(0,0,0,0.9);"
                    >
                      {#if currentPassage.translation === 'Lyrics'}
                        {currentPassage.combined_text}
                      {:else if currentPassage.verses && currentPassage.verses.length > 0}
                        {#each currentPassage.verses as v}
                          <span class="inline"><sup class="text-[0.65em] font-bold opacity-80 mr-1.5 align-super">{v.verse}</sup>{v.text} </span>
                        {/each}
                      {:else}
                        {#if currentPassage.verse_start}
                          <sup class="text-[0.65em] font-bold opacity-80 mr-1.5 align-super">{currentPassage.verse_start}</sup>
                        {/if}
                        {currentPassage.combined_text}
                      {/if}
                    </p>
                    <div class="text-center pt-2 border-t border-neutral-800/80">
                      <span class="text-xl font-bold tracking-wider text-neutral-200 uppercase" style="text-shadow: 0 1px 4px rgba(0,0,0,0.8);">
                        {currentPassage.reference}
                      </span>
                    </div>
                  </div>
                </div>
              {:else}
                <!-- FULL SCREEN EASYWORSHIP / PROPRESENTER PRESENTATION SLIDE -->
                <div
                  class="flex-1 flex flex-col justify-between items-center py-16 mx-auto h-full relative z-10 transition-all duration-300"
                  style="width: {projectorVerseCoverage}%; max-width: {projectorVerseCoverage}%;"
                >
                  <!-- TOP REFERENCE BANNER -->
                  {#if projectorRefPosition.startsWith('top-') && (currentPassage.translation !== 'Media' || currentPassage.combined_text)}
                    <div class="w-full shrink-0 pt-4 pb-6 {projectorRefPosition === 'top-left' ? 'text-left' : projectorRefPosition === 'top-right' ? 'text-right' : 'text-center'}">
                      <span
                        class="text-4xl font-bold tracking-wide font-sans {projectorTheme === 'light' ? 'text-neutral-800' : projectorTheme === 'gold' ? 'text-amber-300' : 'text-neutral-100'}"
                        style="text-shadow: 0 2px 8px rgba(0, 0, 0, 0.9);"
                      >
                        {currentPassage.reference}
                      </span>
                    </div>
                  {:else}
                    <div class="h-16 shrink-0"></div>
                  {/if}

                  <!-- Scripture / Lyrics / Media Center Body -->
                  <div class="flex-1 flex flex-col justify-center items-center text-center w-full py-8">
                    {#if currentPassage.translation === 'Lyrics' || currentPassage.translation === 'Media'}
                      {#if currentPassage.combined_text}
                        <p
                          class="transition-all duration-300 font-bold whitespace-pre-line leading-relaxed tracking-normal"
                          style="font-family: {projectorFontFamilyStyle}; font-size: {dynamicFontSizePx}px; text-shadow: 0 2px 10px rgba(0, 0, 0, 0.9), 0 4px 24px rgba(0, 0, 0, 0.7);"
                        >
                          {currentPassage.combined_text}

                          {#if projectorRefPosition === 'inline' && currentPassage.reference}
                            <span
                              class="inline-block whitespace-nowrap ml-4 font-bold opacity-85 text-[0.8em] tracking-wide {projectorTheme === 'light' ? 'text-neutral-700' : projectorTheme === 'gold' ? 'text-amber-300' : 'text-neutral-200'}"
                              style="text-shadow: 0 2px 8px rgba(0, 0, 0, 0.9);"
                            >
                              — {currentPassage.reference}
                            </span>
                          {/if}
                        </p>
                      {/if}
                    {:else if currentPassage.verses && currentPassage.verses.length > 0}
                      <p
                        class="transition-all duration-300 font-bold whitespace-pre-line leading-relaxed tracking-normal"
                        style="font-family: {projectorFontFamilyStyle}; font-size: {dynamicFontSizePx}px; text-shadow: 0 2px 10px rgba(0, 0, 0, 0.9), 0 4px 24px rgba(0, 0, 0, 0.7);"
                      >
                        {#each currentPassage.verses as v}
                          <span class="inline"><sup class="text-[0.65em] font-bold opacity-80 mr-1.5 align-super">{v.verse}</sup>{v.text} </span>
                        {/each}

                        {#if projectorRefPosition === 'inline'}
                          <span
                            class="inline-block whitespace-nowrap ml-4 font-bold opacity-85 text-[0.8em] tracking-wide {projectorTheme === 'light' ? 'text-neutral-700' : projectorTheme === 'gold' ? 'text-amber-300' : 'text-neutral-200'}"
                            style="text-shadow: 0 2px 8px rgba(0, 0, 0, 0.9);"
                          >
                            — {currentPassage.reference}
                          </span>
                        {/if}
                      </p>
                    {:else}
                      <p
                        class="transition-all duration-300 font-bold whitespace-pre-line leading-relaxed tracking-normal"
                        style="font-family: {projectorFontFamilyStyle}; font-size: {dynamicFontSizePx}px; text-shadow: 0 2px 10px rgba(0, 0, 0, 0.9), 0 4px 24px rgba(0, 0, 0, 0.7);"
                      >
                        {#if currentPassage.verse_start}
                          <sup class="text-[0.65em] font-bold opacity-80 mr-1.5 align-super">{currentPassage.verse_start}</sup>
                        {/if}
                        {currentPassage.combined_text}

                        {#if projectorRefPosition === 'inline'}
                          <span
                            class="inline-block whitespace-nowrap ml-4 font-bold opacity-85 text-[0.8em] tracking-wide {projectorTheme === 'light' ? 'text-neutral-700' : projectorTheme === 'gold' ? 'text-amber-300' : 'text-neutral-200'}"
                            style="text-shadow: 0 2px 8px rgba(0, 0, 0, 0.9);"
                          >
                            — {currentPassage.reference}
                          </span>
                        {/if}
                      </p>
                    {/if}
                  </div>

                  <!-- BOTTOM REFERENCE BANNER -->
                  {#if projectorRefPosition.startsWith('bottom-') && (currentPassage.translation !== 'Media' || currentPassage.combined_text)}
                    <div class="w-full shrink-0 mt-auto pb-12 {projectorRefPosition === 'bottom-left' ? 'text-left' : projectorRefPosition === 'bottom-right' ? 'text-right' : 'text-center'}">
                      <span
                        class="text-4xl font-bold tracking-wide font-sans {projectorTheme === 'light' ? 'text-neutral-800' : projectorTheme === 'gold' ? 'text-amber-300' : 'text-neutral-100'}"
                        style="text-shadow: 0 2px 8px rgba(0, 0, 0, 0.9);"
                      >
                        {currentPassage.reference}
                      </span>
                    </div>
                  {:else}
                    <div class="h-16 shrink-0"></div>
                  {/if}
                </div>
              {/if}
            </div>
          {/if}

          <!-- Badge in corner -->
          <div class="absolute bottom-1.5 right-1.5 px-1.5 py-0.5 rounded bg-black/60 text-[8px] font-mono text-neutral-400 border border-neutral-800 pointer-events-none z-10">
            1:1 1080p Mirror
          </div>
        </div>
      </div>

      <!-- Quick Manual Scripture Search & Library -->
      <div class="flex-1 p-5 flex flex-col gap-3 overflow-hidden">
        <div class="flex items-center justify-between">
          <span class="text-xs font-semibold tracking-wide uppercase text-neutral-400">Manual Lookup (⌘K)</span>
          <span class="text-[11px] text-neutral-500">e.g. John 3:16 or Romans 8:28</span>
        </div>

        <!-- Search Input -->
        <div class="relative flex items-center">
          <Search class="w-3.5 h-3.5 absolute left-3 {effectiveTheme === 'light' ? 'text-stone-400' : 'text-neutral-500'}" />
          <input
            type="text"
            bind:value={searchQuery}
            onkeydown={(e) => e.key === 'Enter' && handleSearch()}
            placeholder="Type book, reference, or keywords..."
            class="w-full {effectiveTheme === 'light' ? 'bg-white border-stone-200 text-stone-900 placeholder-stone-400 focus:border-amber-500' : 'bg-neutral-950 border-neutral-800 text-neutral-200 placeholder-neutral-600 focus:border-neutral-700'} rounded pl-9 pr-16 py-2 text-xs border focus:outline-none"
          />
          <button
            onclick={handleSearch}
            class="absolute right-1.5 px-2 py-1 rounded {effectiveTheme === 'light' ? 'bg-stone-100 border-stone-200 text-stone-700 hover:bg-stone-200 hover:text-stone-900' : 'bg-neutral-900 border-neutral-800 text-neutral-400 hover:text-neutral-200'} border text-[10px] font-medium transition-colors"
          >
            Search
          </button>
        </div>

        <!-- Search Results / Preset Scriptures List -->
        <div class="flex-1 overflow-y-auto flex flex-col gap-1.5 border {effectiveTheme === 'light' ? 'bg-stone-100/70 border-stone-200' : 'bg-neutral-950/40 border-neutral-800/80'} rounded p-2">
          {#if searchResults.length > 0}
            {#each searchResults as verse}
              <button
                onclick={() => presentSearchedVerse(verse)}
                class="p-2.5 rounded {effectiveTheme === 'light' ? 'bg-white border-stone-200 hover:bg-stone-50 hover:border-stone-300 shadow-xs' : 'bg-neutral-900/60 border-neutral-800/80 hover:border-neutral-700 hover:bg-neutral-800/60'} border text-left transition-colors group flex flex-col gap-1"
              >
                <div class="flex items-center justify-between">
                  <span class="text-xs font-bold {effectiveTheme === 'light' ? 'text-stone-900 group-hover:text-indigo-600' : 'text-neutral-200 group-hover:text-white'}">
                    {verse.book} {verse.chapter}:{verse.verse}
                  </span>
                  <span class="text-[10px] font-mono {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'}">Click to Present</span>
                </div>
                <p class="text-xs {effectiveTheme === 'light' ? 'text-stone-600' : 'text-neutral-400'} font-serif line-clamp-2 italic">
                  "{verse.text}"
                </p>
              </button>
            {/each}
          {:else}
            <!-- Default Quick Picks -->
            <div class="p-1">
              <span class="text-[10px] uppercase font-semibold {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-600'} tracking-wider">Quick Key Verses</span>
              <div class="flex flex-col gap-1.5 mt-2">
                {#each [
                  { ref: "John 3:16", text: "For God so loved the world, that he gave his only begotten Son..." },
                  { ref: "Romans 8:28", text: "And we know that all things work together for good to them that love God..." },
                  { ref: "Psalms 23:1", text: "The LORD is my shepherd; I shall not want." },
                  { ref: "Philippians 4:13", text: "I can do all things through Christ which strengtheneth me." },
                  { ref: "1 Corinthians 13:4", text: "Charity suffereth long, and is kind; charity envieth not..." }
                ] as item}
                  <button
                    onclick={() => selectQuickVerse(item.ref)}
                    class="p-2.5 rounded {effectiveTheme === 'light' ? 'bg-white border-stone-200 hover:border-stone-300 hover:bg-stone-50 shadow-xs' : 'bg-neutral-900/40 border-neutral-800/60 hover:border-neutral-700 hover:bg-neutral-900'} border text-left transition-colors flex flex-col gap-0.5"
                  >
                    <span class="text-xs font-semibold {effectiveTheme === 'light' ? 'text-stone-900' : 'text-neutral-300'}">{item.ref}</span>
                    <span class="text-[11px] {effectiveTheme === 'light' ? 'text-stone-600' : 'text-neutral-500'} truncate font-serif italic">{item.text}</span>
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </main>

  <!-- IDE-STYLE BOTTOM DOCK (Cursor / VS Code inspired) -->
  <nav class="h-10 {effectiveTheme === 'light' ? 'bg-white border-stone-200' : 'bg-[#0c0c10] border-neutral-800/80'} border-t px-4 flex items-center justify-between text-xs shrink-0 select-none z-30">
    <!-- Left: Activity Views -->
    <div class="flex items-center gap-1">
      <button
        onclick={() => { activeDockView = 'scripture'; showThemePopover = false; }}
        class="flex items-center gap-1.5 px-3 py-1.5 rounded text-xs transition-colors {activeDockView === 'scripture' ? (effectiveTheme === 'light' ? 'bg-stone-200 text-stone-900 font-semibold shadow-xs' : 'bg-neutral-800 text-neutral-100 font-semibold shadow-sm') : (effectiveTheme === 'light' ? 'text-stone-600 hover:text-stone-900 hover:bg-stone-100' : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-850')}"
        title="Scripture & Voice Recognition Console"
      >
        <BookOpen class="w-3.5 h-3.5 {activeDockView === 'scripture' ? 'text-indigo-500' : 'text-neutral-400'}" />
        <span>Scripture</span>
      </button>

      <button
        onclick={() => { activeDockView = 'songs'; showThemePopover = false; }}
        class="flex items-center gap-1.5 px-3 py-1.5 rounded text-xs transition-colors {activeDockView === 'songs' ? (effectiveTheme === 'light' ? 'bg-stone-200 text-stone-900 font-semibold shadow-xs' : 'bg-neutral-800 text-neutral-100 font-semibold shadow-sm') : (effectiveTheme === 'light' ? 'text-stone-600 hover:text-stone-900 hover:bg-stone-100' : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-850')}"
        title="Songs & Worship Lyrics Presenter"
      >
        <Music class="w-3.5 h-3.5 {activeDockView === 'songs' ? 'text-amber-500' : 'text-neutral-400'}" />
        <span>Songs</span>
      </button>

      <button
        onclick={() => { activeDockView = 'media'; showThemePopover = false; }}
        class="flex items-center gap-1.5 px-3 py-1.5 rounded text-xs transition-colors {activeDockView === 'media' ? (effectiveTheme === 'light' ? 'bg-stone-200 text-stone-900 font-semibold shadow-xs' : 'bg-neutral-800 text-neutral-100 font-semibold shadow-sm') : (effectiveTheme === 'light' ? 'text-stone-600 hover:text-stone-900 hover:bg-stone-100' : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-850')}"
        title="Media & Motion Backdrops Studio"
      >
        <Image class="w-3.5 h-3.5 {activeDockView === 'media' ? 'text-rose-500' : 'text-neutral-400'}" />
        <span>Media</span>
      </button>
    </div>

    <!-- Right: Theme & Settings -->
    <div class="flex items-center gap-1.5 relative">
      <!-- Theme Switcher Button -->
      <button
        onclick={() => { showThemePopover = !showThemePopover; }}
        class="flex items-center gap-1.5 px-2.5 py-1.5 rounded text-xs transition-colors {showThemePopover ? (effectiveTheme === 'light' ? 'bg-stone-200 text-stone-900 font-medium' : 'bg-neutral-800 text-neutral-100 font-medium') : (effectiveTheme === 'light' ? 'text-stone-600 hover:text-stone-900 hover:bg-stone-100' : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-850')}"
        title="Display Theme Picker"
      >
        <Palette class="w-3.5 h-3.5 text-emerald-500" />
        <span class="capitalize">Stage: {projectorTheme === 'dark' ? 'Blue' : projectorTheme.startsWith('data:') || projectorTheme.startsWith('http') || projectorTheme.startsWith('url(') ? 'Custom' : projectorTheme}</span>
      </button>

      <!-- Settings Activity Button (VS Code style) -->
      <button
        onclick={() => {
          if (activeDockView === 'settings') {
            activeDockView = 'scripture';
          } else {
            activeDockView = 'settings';
          }
          showThemePopover = false;
        }}
        class="flex items-center gap-1.5 px-2.5 py-1.5 rounded text-xs transition-colors {activeDockView === 'settings' ? (effectiveTheme === 'light' ? 'bg-stone-200 text-stone-900 font-semibold' : 'bg-neutral-800 text-neutral-100 font-semibold') : (effectiveTheme === 'light' ? 'text-stone-600 hover:text-stone-900 hover:bg-stone-100' : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-850')}"
        title="Settings & Preferences"
      >
        <Settings class="w-3.5 h-3.5 {activeDockView === 'settings' ? 'text-indigo-500 rotate-45' : 'text-neutral-400'} transition-transform duration-200" />
        <span>Settings</span>
      </button>

      <!-- Floating Theme Popover (anchored right above dock) -->
      {#if showThemePopover}
        <div class="absolute bottom-11 right-12 w-64 rounded-xl {effectiveTheme === 'light' ? 'bg-white border-stone-200 shadow-2xl' : 'bg-neutral-900 border-neutral-700/80 shadow-2xl'} border p-2 z-50 flex flex-col gap-1 backdrop-blur-md">
          <div class="px-2 py-1 border-b {effectiveTheme === 'light' ? 'border-stone-100 text-stone-500' : 'border-neutral-800 text-neutral-400'} text-[11px] font-semibold uppercase tracking-wider flex items-center justify-between">
            <span>Projector Stage Theme</span>
            <Palette class="w-3.5 h-3.5 text-emerald-500" />
          </div>
          <div class="flex flex-col gap-0.5 max-h-72 overflow-y-auto">
            {#each [
              { id: 'dark', label: 'Atmospheric Blue', desc: 'Celestial blue gradient glow' },
              { id: 'celestial', label: 'Celestial Violet', desc: 'Deep purple aurora gradient' },
              { id: 'emerald', label: 'Emerald Sanctuary', desc: 'Deep evergreen sacred worship aura' },
              { id: 'crimson', label: 'Crimson Majesty', desc: 'Reverent ruby and wine ambient glow' },
              { id: 'ocean', label: 'Oceanic Azure', desc: 'Deep marine turquoise and sapphire' },
              { id: 'sunset', label: 'Golden Sunset', desc: 'Warm twilight amber & coral dusk' },
              { id: 'gold', label: 'Midnight Gold', desc: 'Deep navy with warm amber text' },
              { id: 'minimal-black', label: 'Minimal Black', desc: 'Pure black background' },
              { id: 'light', label: 'Studio Light', desc: 'Clean white background with dark text' },
              { id: 'lower-third', label: 'Lower-Third', desc: 'Transparent broadcast overlay' }
            ] as th}
              <button
                onclick={() => { setTheme(th.id); showThemePopover = false; }}
                class="w-full text-left px-2.5 py-1.5 rounded text-xs transition-colors flex items-center justify-between {projectorTheme === th.id ? (effectiveTheme === 'light' ? 'bg-stone-100 text-stone-900 font-medium' : 'bg-neutral-800 text-neutral-100 font-medium') : (effectiveTheme === 'light' ? 'text-stone-700 hover:bg-stone-50' : 'text-neutral-300 hover:bg-neutral-800/60 hover:text-white')}"
              >
                <div>
                  <p class="font-medium leading-none">{th.label}</p>
                  <p class="text-[10px] {effectiveTheme === 'light' ? 'text-stone-500' : 'text-neutral-500'} mt-0.5">{th.desc}</p>
                </div>
                {#if projectorTheme === th.id}
                  <Check class="w-3.5 h-3.5 text-emerald-500 shrink-0" />
                {/if}
              </button>
            {/each}
          </div>
          <div class="pt-1.5 mt-1 border-t {effectiveTheme === 'light' ? 'border-stone-100' : 'border-neutral-800'}">
            <button
              onclick={() => { activeDockView = 'media'; showThemePopover = false; }}
              class="w-full text-left px-2.5 py-1.5 rounded text-xs {effectiveTheme === 'light' ? 'text-rose-600 hover:bg-rose-50' : 'text-rose-400 hover:bg-rose-950/40'} font-medium transition-colors flex items-center justify-between"
            >
              <span class="flex items-center gap-1.5">
                <Image class="w-3.5 h-3.5" />
                Open Media Studio
              </span>
              <ChevronRight class="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      {/if}
    </div>
  </nav>
</div>
