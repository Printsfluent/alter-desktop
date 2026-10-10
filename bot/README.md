# Telegram bot

This bot runs the app's Instagram/Threads media pipelines. It accepts videos and
images, returns 1–5 requested variants, and defaults video strength to
`medium`. After an upload, it asks how many variants to make (1–5), then asks
whether the media contains written captions or other overlaid text. A “yes”
keeps every output unmirrored; a “no” enables horizontal mirroring. `/cancel`
clears a pending upload.

The renderers remove the source media metadata and write fresh output metadata.

## Configure and run

1. Create a bot with [@BotFather](https://t.me/BotFather) and copy its token.
2. Set `TELEGRAM_BOT_TOKEN` to that token.
3. Set `ALLOWED_USERS` to a comma-separated list of Telegram numeric user IDs
   (use `/id` with the bot to find yours). Keep this list restricted to people
   authorized to use the bot.
4. Install `ffmpeg` and `ffprobe` on the host, or make them available in PATH.
5. From `oopsie/bot`, export your settings and run the bot:

   ```bash
   set -a
   source .env.local
   set +a
   cargo run --release
   ```

   `.env.local` is ignored by Git. For a hosted deployment, add the same values
   as environment secrets in the hosting provider instead.

Optional settings:

- `SPOOF_STRENGTH`: `minimal`, `medium`, `hard`, or `extreme`; defaults to
  `medium`.
- `SPOOF_SPEED`, `SPOOF_BW_PCT`, `SPOOF_MIRROR_PCT`, and `SPOOF_TRIM_SECS`:
  video pipeline settings, with the same defaults as the desktop app.
- `PORT`: optionally starts a basic HTTP health endpoint for hosting platforms.

The bot accepts Telegram uploads up to 20 MB (Bot API download limit). Send
images as files to avoid Telegram photo recompression. Image inputs are JPG,
PNG, WEBP, and TIFF; video inputs are MP4, MOV, M4V, MKV, WEBM, and AVI.

Never commit the bot token. Set it through your host's secret/environment
configuration.
