<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { Maximize, Minimize } from 'lucide-svelte';

  interface ScriptureVerse {
    book: string;
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

  let currentPassage = $state<ScripturePassage | null>(null);
  let isCleared = $state(false);
  let isBlackout = $state(false);
  let theme = $state<string>('dark');
  let fontFamily = $state<string>('serif');
  let fontSize = $state<string>('auto');
  let refPosition = $state<string>('bottom-center');
  let verseCoverage = $state<number>(85);
  let autoFit = $state<boolean>(true);
  let autoGrow = $state<boolean>(true);
  let autoShrink = $state<boolean>(true);
  let minFontSize = $state<number>(28);
  let maxFontSize = $state<number>(84);
  let isFullscreen = $state(false);
  let showControls = $state(false);
  let cursorHidden = $state(false);
  let controlsTimeout: any = null;
  let cursorTimeout: any = null;

  let unlisteners: UnlistenFn[] = [];

  onMount(async () => {
    try {
      const status: any = await invoke('get_status');
      currentPassage = status.current_passage;
      isCleared = status.projector_clear;
      isBlackout = status.projector_blackout;
      theme = status.projector_theme || 'dark';
      fontFamily = (status.projector_font_family as string) || 'serif';
      fontSize = (status.projector_font_size as string) || 'auto';
      refPosition = status.projector_ref_position || 'bottom-center';
      verseCoverage = Number(status.projector_verse_coverage) || 85;
      autoFit = status.projector_auto_fit !== undefined ? status.projector_auto_fit : true;
      autoGrow = status.projector_auto_grow !== undefined ? status.projector_auto_grow : true;
      autoShrink = status.projector_auto_shrink !== undefined ? status.projector_auto_shrink : true;
      minFontSize = Number(status.projector_min_font_size) || 28;
      maxFontSize = Number(status.projector_max_font_size) || 84;

      const u1 = await listen<ScripturePassage>('present-slide', (event) => {
        console.log('Projector received present-slide:', event.payload);
        currentPassage = event.payload;
        isCleared = false;
        isBlackout = false;
      });
      unlisteners.push(u1);

      const u2 = await listen('projector-clear', () => {
        isCleared = true;
      });
      unlisteners.push(u2);

      const u3 = await listen<boolean>('projector-blackout', (event) => {
        isBlackout = event.payload;
      });
      unlisteners.push(u3);

      const u4 = await listen<string>('projector-theme', (event) => {
        theme = event.payload as any;
      });
      unlisteners.push(u4);

      const u5 = await listen<any>('projector-typography', (event) => {
        const ff = event.payload.fontFamily || event.payload.font_family;
        const fs = event.payload.fontSize || event.payload.font_size;
        if (ff) fontFamily = String(ff);
        if (fs) fontSize = String(fs);
      });
      unlisteners.push(u5);

      const u6 = await listen<any>('projector-layout', (event) => {
        const p = event.payload;
        if (p.refPosition !== undefined) refPosition = p.refPosition;
        if (p.verseCoverage !== undefined) verseCoverage = Number(p.verseCoverage);
        if (p.autoFit !== undefined) autoFit = Boolean(p.autoFit);
        if (p.autoGrow !== undefined) autoGrow = Boolean(p.autoGrow);
        if (p.autoShrink !== undefined) autoShrink = Boolean(p.autoShrink);
        if (p.minFontSize !== undefined) minFontSize = Number(p.minFontSize);
        if (p.maxFontSize !== undefined) maxFontSize = Number(p.maxFontSize);
      });
      unlisteners.push(u6);
    } catch (e) {
      console.warn('Projector initialization:', e);
    }

    window.addEventListener('keydown', handleKey);
  });

  onDestroy(() => {
    unlisteners.forEach((u) => u());
    if (controlsTimeout) clearTimeout(controlsTimeout);
    if (cursorTimeout) clearTimeout(cursorTimeout);
    window.removeEventListener('keydown', handleKey);
  });

  function handleKey(e: KeyboardEvent) {
    if (e.key === 'f' || e.key === 'F11') {
      toggleFullscreen();
    } else if (e.key === 'Escape' && document.fullscreenElement) {
      document.exitFullscreen().catch(() => {});
      isFullscreen = false;
    }
  }

  function toggleFullscreen() {
    if (!document.fullscreenElement) {
      document.documentElement.requestFullscreen().catch(() => {});
      isFullscreen = true;
    } else {
      document.exitFullscreen().catch(() => {});
      isFullscreen = false;
    }
  }

  function handleMouseMove() {
    cursorHidden = false;
    showControls = true;
    if (controlsTimeout) clearTimeout(controlsTimeout);
    if (cursorTimeout) clearTimeout(cursorTimeout);
    controlsTimeout = setTimeout(() => {
      showControls = false;
    }, 2500);
    cursorTimeout = setTimeout(() => {
      cursorHidden = true;
    }, 2000);
  }

  // Dynamic Font Family (Supports System Fonts + Fallbacks)
  let fontFamilyStyle = $derived.by(() => {
    if (!fontFamily || fontFamily === 'serif') return 'Georgia, Cambria, "Times New Roman", serif';
    if (fontFamily === 'sans') return 'system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
    if (fontFamily === 'slab') return '"SF Mono", Menlo, Courier, monospace';
    return `"${fontFamily}", system-ui, -apple-system, sans-serif`;
  });

  // Dynamic font sizing engine (FreeShow style Auto-Fit with Auto-Grow and Auto-Shrink)
  function computeDynamicFontSize(
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
    return computeDynamicFontSize(len, fontSize, autoFit, autoGrow, autoShrink, minFontSize, maxFontSize);
  });

  // Dynamic Theme Background Styles (Atmospheric Worship Backdrops)
  let themeBackgroundStyle = $derived.by(() => {
    if (isBlackout) return 'background: #000000; color: #ffffff;';
    if (theme === 'light') return 'background: #f8fafc; color: #0f172a;';
    if (theme === 'lower-third') return 'background: transparent; color: #ffffff;';
    if (theme === 'minimal-black' || theme === 'black') return 'background: #000000; color: #ffffff;';
    if (theme === 'gold') {
      return 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #1c1a2e 0%, #0c0b16 55%, #05040a 100%); color: #fef08a;';
    }
    if (theme === 'celestial') {
      return 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #351c54 0%, #1a0f30 55%, #0a0514 100%); color: #ffffff;';
    }
    if (theme === 'emerald') {
      return 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #063826 0%, #031c13 55%, #010a07 100%); color: #a7f3d0;';
    }
    if (theme === 'crimson') {
      return 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #420d18 0%, #20060c 55%, #0d0205 100%); color: #fecdd3;';
    }
    if (theme === 'ocean') {
      return 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #0a3a40 0%, #051d20 55%, #020c0e 100%); color: #a5f3fc;';
    }
    if (theme === 'sunset') {
      return 'background: radial-gradient(ellipse 90% 70% at 50% 35%, #3d1c24 0%, #1d0e14 55%, #0b0508 100%); color: #fed7aa;';
    }
    if (theme.startsWith('url(') || theme.startsWith('data:') || theme.startsWith('http')) {
      const url = theme.startsWith('url(') ? theme : `url("${theme}")`;
      return `background: ${url} center/cover no-repeat, #000000; color: #ffffff;`;
    }
    // Default 'dark' / 'atmospheric': Gorgeous worship ambient celestial blue gradient as seen in the screenshots!
    return 'background: radial-gradient(ellipse 95% 75% at 50% 35%, #183d63 0%, #0d223c 55%, #050e1a 100%); color: #ffffff;';
  });
</script>

<!-- Projector Root Container -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  onmousemove={handleMouseMove}
  ondblclick={toggleFullscreen}
  class="w-screen h-screen overflow-hidden flex flex-col transition-all duration-300 relative select-none {cursorHidden ? 'cursor-none' : 'cursor-default'}"
  style="{themeBackgroundStyle}"
>
  <!-- Atmospheric Ambient Glow Lighting (Soft worship background depth) -->
  {#if !isBlackout && theme !== 'lower-third' && theme !== 'light' && theme !== 'minimal-black' && theme !== 'black'}
    <div class="absolute inset-0 pointer-events-none overflow-hidden">
      {#if theme === 'celestial'}
        <div class="absolute -top-[20%] -left-[10%] w-[60vw] h-[60vw] rounded-full bg-purple-600/15 blur-[120px]"></div>
        <div class="absolute top-[30%] -right-[15%] w-[55vw] h-[55vw] rounded-full bg-fuchsia-600/15 blur-[130px]"></div>
      {:else if theme === 'gold'}
        <div class="absolute -top-[20%] left-[20%] w-[60vw] h-[60vw] rounded-full bg-amber-600/10 blur-[130px]"></div>
      {:else if theme === 'emerald'}
        <div class="absolute -top-[20%] -left-[10%] w-[60vw] h-[60vw] rounded-full bg-emerald-600/15 blur-[120px]"></div>
        <div class="absolute top-[30%] -right-[15%] w-[55vw] h-[55vw] rounded-full bg-teal-600/15 blur-[130px]"></div>
      {:else if theme === 'crimson'}
        <div class="absolute -top-[20%] -left-[10%] w-[60vw] h-[60vw] rounded-full bg-rose-600/15 blur-[120px]"></div>
        <div class="absolute top-[30%] -right-[15%] w-[55vw] h-[55vw] rounded-full bg-red-700/15 blur-[130px]"></div>
      {:else if theme === 'ocean'}
        <div class="absolute -top-[20%] -left-[10%] w-[60vw] h-[60vw] rounded-full bg-cyan-600/15 blur-[120px]"></div>
        <div class="absolute top-[30%] -right-[15%] w-[55vw] h-[55vw] rounded-full bg-teal-600/15 blur-[130px]"></div>
      {:else if theme === 'sunset'}
        <div class="absolute -top-[20%] -left-[10%] w-[60vw] h-[60vw] rounded-full bg-orange-600/15 blur-[120px]"></div>
        <div class="absolute top-[30%] -right-[15%] w-[55vw] h-[55vw] rounded-full bg-amber-600/15 blur-[130px]"></div>
      {:else if !theme.startsWith('url(') && !theme.startsWith('data:') && !theme.startsWith('http')}
        <!-- Atmospheric Blue/Teal (Screenshot Style) -->
        <div class="absolute -top-[20%] -left-[10%] w-[60vw] h-[60vw] rounded-full bg-cyan-600/15 blur-[120px]"></div>
        <div class="absolute top-[30%] -right-[15%] w-[55vw] h-[55vw] rounded-full bg-blue-600/15 blur-[130px]"></div>
        <div class="absolute -bottom-[20%] left-[20%] w-[50vw] h-[50vw] rounded-full bg-indigo-700/15 blur-[140px]"></div>
      {/if}
    </div>
  {/if}

  {#if !isBlackout && !isCleared && currentPassage}
    {#if theme === 'lower-third'}
      <!-- LOWER-THIRD BROADCAST OVERLAY -->
      <div class="mt-auto w-full p-8 pb-12 bg-gradient-to-t from-black/95 via-black/80 to-transparent flex flex-col items-center relative z-10">
        <div class="w-full max-w-5xl rounded-2xl bg-neutral-950/95 border border-neutral-800/80 p-6 shadow-2xl flex flex-col gap-3 backdrop-blur-md">
          <p
            class="text-lg md:text-xl text-neutral-100 font-bold leading-relaxed whitespace-pre-line text-center"
            style="font-family: {fontFamilyStyle}; font-size: calc(({Math.min(dynamicFontSizePx, 36)} / 1920) * 100vw); text-shadow: 0 2px 8px rgba(0,0,0,0.9);"
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
          <div class="text-center pt-1 border-t border-neutral-800/80">
            <span class="text-sm font-bold tracking-wider text-neutral-200 uppercase" style="text-shadow: 0 1px 4px rgba(0,0,0,0.8);">
              {currentPassage.reference}
            </span>
          </div>
        </div>
      </div>
    {:else}
      <!-- FULL SCREEN EASYWORSHIP / PROPRESENTER PRESENTATION SLIDE -->
      <div
        class="flex-1 flex flex-col justify-between items-center py-8 md:py-14 mx-auto h-full relative z-10 transition-all duration-300"
        style="width: {verseCoverage}%; max-width: {verseCoverage}%;"
      >
        <!-- TOP REFERENCE BANNER (When pinned to top) -->
        {#if refPosition.startsWith('top-') && (currentPassage.translation !== 'Media' || currentPassage.combined_text)}
          <div class="w-full shrink-0 pt-2 pb-4 {refPosition === 'top-left' ? 'text-left' : refPosition === 'top-right' ? 'text-right' : 'text-center'}">
            <span
              class="font-bold tracking-wide font-sans {theme === 'light' ? 'text-neutral-800' : theme === 'gold' ? 'text-amber-300' : 'text-neutral-100'}"
              style="font-size: calc((32 / 1920) * 100vw); text-shadow: 0 2px 8px rgba(0, 0, 0, 0.9);"
            >
              {currentPassage.reference}
            </span>
          </div>
        {:else}
          <div class="h-6 md:h-12 shrink-0"></div>
        {/if}

        <!-- Center Stage Scripture / Song Lyrics Body -->
        <div class="flex-1 flex flex-col justify-center items-center text-center w-full py-4">
          {#if currentPassage.translation === 'Lyrics' || currentPassage.translation === 'Media'}
            {#if currentPassage.combined_text}
              <p
                class="transition-all duration-300 font-bold whitespace-pre-line leading-relaxed tracking-normal"
                style="font-family: {fontFamilyStyle}; font-size: calc(({dynamicFontSizePx} / 1920) * 100vw); text-shadow: 0 2px 10px rgba(0, 0, 0, 0.9), 0 4px 24px rgba(0, 0, 0, 0.7);"
              >
                {currentPassage.combined_text}
              </p>
            {/if}
          {:else}
            <p
              class="transition-all duration-300 font-bold whitespace-pre-line leading-relaxed tracking-normal"
              style="font-family: {fontFamilyStyle}; font-size: calc(({dynamicFontSizePx} / 1920) * 100vw); text-shadow: 0 2px 10px rgba(0, 0, 0, 0.9), 0 4px 24px rgba(0, 0, 0, 0.7);"
            >
              {#if currentPassage.verses && currentPassage.verses.length > 0}
                {#each currentPassage.verses as v}
                  <span class="inline"><sup class="text-[0.65em] font-bold opacity-80 mr-1.5 align-super">{v.verse}</sup>{v.text} </span>
                {/each}
              {:else}
                {#if currentPassage.verse_start}
                  <sup class="text-[0.65em] font-bold opacity-80 mr-1.5 align-super">{currentPassage.verse_start}</sup>
                {/if}
                {currentPassage.combined_text}
              {/if}

              {#if refPosition === 'inline'}
                <span
                  class="inline-block whitespace-nowrap ml-3 font-bold opacity-85 text-[0.8em] tracking-wide {theme === 'light' ? 'text-neutral-700' : theme === 'gold' ? 'text-amber-300' : 'text-neutral-200'}"
                  style="text-shadow: 0 2px 8px rgba(0, 0, 0, 0.9);"
                >
                  — {currentPassage.reference}
                </span>
              {/if}
            </p>
          {/if}
        </div>

        <!-- BOTTOM REFERENCE BANNER (When pinned to bottom) -->
        {#if refPosition.startsWith('bottom-') && (currentPassage.translation !== 'Media' || currentPassage.combined_text)}
          <div class="w-full shrink-0 mt-auto pb-4 md:pb-8 {refPosition === 'bottom-left' ? 'text-left' : refPosition === 'bottom-right' ? 'text-right' : 'text-center'}">
            <span
              class="font-bold tracking-wide font-sans {theme === 'light' ? 'text-neutral-800' : theme === 'gold' ? 'text-amber-300' : 'text-neutral-100'}"
              style="font-size: calc((32 / 1920) * 100vw); text-shadow: 0 2px 8px rgba(0, 0, 0, 0.9);"
            >
              {currentPassage.reference}
            </span>
          </div>
        {:else}
          <div class="h-6 md:h-12 shrink-0"></div>
        {/if}
      </div>
    {/if}
  {/if}

  <!-- Hover Floating Utility Controls -->
  {#if showControls}
    <div class="absolute top-4 right-4 flex items-center gap-2 bg-neutral-950/80 border border-neutral-800 rounded-full px-3 py-1.5 backdrop-blur-md shadow-lg transition-opacity duration-200">
      <span class="text-[10px] font-mono text-neutral-400">Double-click or press F for Fullscreen</span>
      <button
        onclick={toggleFullscreen}
        class="text-neutral-300 hover:text-white transition-colors"
        title="Toggle Fullscreen"
      >
        {#if isFullscreen}
          <Minimize class="w-4 h-4" />
        {:else}
          <Maximize class="w-4 h-4" />
        {/if}
      </button>
    </div>
  {/if}
</div>
