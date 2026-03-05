import { convertFileSrc } from '@tauri-apps/api/core';

let audioCtx = null;
let audioNodes = [];
let masterGain = null;

export function stopMicrowaveAudio(closeContext = true) {
  for (const node of audioNodes) {
    try {
      if (node instanceof HTMLMediaElement) {
        node.pause();
        node.currentTime = 0;
      } else {
        node.stop();
      }
    } catch (e) {}
  }
  audioNodes = [];

  if (closeContext && audioCtx) {
    try { audioCtx.close(); } catch (e) {}
    audioCtx = null;
    masterGain = null;
  }
}

export function startMicrowaveAudio(config) {
  if (config.microwave?.muted) return;
  if (audioCtx || audioNodes.length > 0) return;

  try {
    audioCtx = new (window.AudioContext || window.webkitAudioContext)();
    masterGain = audioCtx.createGain();
    masterGain.gain.value = 1;
    masterGain.connect(audioCtx.destination);

    const humAssetId = config.microwave?.hum_id;
    const humAsset = config.assets?.find(a => a.id === humAssetId);

    if (humAsset) {
      const audio = new Audio(convertFileSrc(humAsset.path));
      audio.loop = true;
      const source = audioCtx.createMediaElementSource(audio);
      source.connect(masterGain);
      audio.play();
      audioNodes = [audio];
    } else {
      const bufferSize = audioCtx.sampleRate * 2;
      const noiseBuffer = audioCtx.createBuffer(1, bufferSize, audioCtx.sampleRate);
      const data = noiseBuffer.getChannelData(0);
      let brown = 0;
      for (let i = 0; i < bufferSize; i++) {
        const white = Math.random() * 2 - 1;
        brown = (brown + (0.02 * white)) / 1.02;
        data[i] = (brown * 0.7 + white * 0.3) * 3.5;
      }
      const noise = audioCtx.createBufferSource();
      noise.buffer = noiseBuffer;
      noise.loop = true;

      const noiseFilter = audioCtx.createBiquadFilter();
      noiseFilter.type = 'lowpass';
      noiseFilter.frequency.value = 400;
      noiseFilter.Q.value = 0.5;

      const noiseGain = audioCtx.createGain();
      noiseGain.gain.value = 0.08;

      noise.connect(noiseFilter);
      noiseFilter.connect(noiseGain);
      noiseGain.connect(masterGain);
      noise.start();

      const hum = audioCtx.createOscillator();
      hum.type = 'sine';
      hum.frequency.value = 60;
      const humGain = audioCtx.createGain();
      humGain.gain.value = 0.02;
      hum.connect(humGain);
      humGain.connect(masterGain);
      hum.start();

      audioNodes = [noise, hum];
    }
  } catch (e) {
    console.warn('Audio not available:', e);
  }
}

export function playDing(config) {
  if (!audioCtx || config.microwave?.muted) return;
  
  try {
    const dingAssetId = config.microwave?.ding_id;
    const dingAsset = config.assets?.find(a => a.id === dingAssetId);

    if (dingAsset) {
      const ding = new Audio(convertFileSrc(dingAsset.path));
      const source = audioCtx.createMediaElementSource(ding);
      source.connect(masterGain);
      ding.play();
    } else {
      const now = audioCtx.currentTime;
      const g = audioCtx.createGain();
      g.gain.setValueAtTime(0, now);
      g.gain.linearRampToValueAtTime(0.2, now + 0.02);
      g.gain.exponentialRampToValueAtTime(0.001, now + 0.8);
      g.connect(masterGain);

      const o = audioCtx.createOscillator();
      o.type = 'sine';
      o.frequency.setValueAtTime(1200, now);
      o.frequency.exponentialRampToValueAtTime(1180, now + 0.8);
      o.connect(g);
      o.start(now);
      o.stop(now + 0.8);
    }
  } catch(e) {}
}

export function getAudioCtx() {
  return audioCtx;
}
