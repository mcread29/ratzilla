# Canvas video example

Place a browser-compatible video at `assets/sample.mp4`, then run:

```sh
cd examples/canvas_video
trunk serve
```

No media binary is committed. A short MP4/WebM you created or a clearly licensed sample is recommended. A missing or unsupported file is reported in the Ratatui UI rather than fetched from a third-party service.

Use `?backend=canvas` or `?backend=webgl2` to select a backend. The DOM backend cannot paint video overlays.

Click/focus the terminal, then press **Space** to play/pause. Browsers require playback to begin from visitor interaction. **Left/Right** seek by five seconds and **M** toggles mute.
