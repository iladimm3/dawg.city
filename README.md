# dawg.city 🐶🏙️

**dawg.city** is a high-performance web application designed to instantly detect AI-generated content in YouTube videos. Built with a Rust backend and a polished, single-file Tailwind CSS frontend, it leverages the Sightengine GenAI detection API to provide accurate verdicts.

## Features
- 🚀 **Fast Rust Backend**: Powered by `vercel_runtime` and `tokio`.
- 🎨 **Premium UI**: Single-file `index.html` with Tailwind CSS, dark mode, and smooth animations.
- 📺 **YouTube Support**: Works with standard videos, Shorts, and Live streams.
- 🛡️ **Production Ready**: Includes CORS handling and Vercel-specific optimizations.

## Project Structure
```text
dawg.city/
├── index.html       # Single-file frontend (Tailwind + Vanilla JS)
├── api/
│   └── analyze/
│       ├── Cargo.toml  # Rust dependencies
│       └── src/
│           └── main.rs # Serverless Rust function for Vercel
├── vercel.json      # Vercel configuration
└── README.md        # This file
```

## Deployment Steps

1.  **Fork/Clone**: Create a new repository on GitHub and push these files.
2.  **Import to Vercel**:
    - Go to [vercel.com](https://vercel.com) and click **"Add New" -> "Project"**.
    - Import your repository.
    - Vercel will automatically detect the Rust runtime.
3.  **Environment Variables**:
    In your Vercel project settings, add the following Environment Variables:
    - `SIGHTENGINE_API_USER`: Your Sightengine API User ID.
    - `SIGHTENGINE_API_SECRET`: Your Sightengine API Secret.
4.  **Custom Domain**:
    - Point `dawg.city` to Vercel in the "Domains" tab.
5.  **Deploy**: Hit "Deploy" and you're live!

## How it Works
1.  User pastes a YouTube URL.
2.  Frontend sends a `POST` request to `/api/analyze`.
3.  Backend extracts the YouTube Video ID and fetches the high-res thumbnail.
4.  The thumbnail is sent to Sightengine's `genai` model.
5.  The result is mapped to a verdict (`ai_generated`, `likely_real`, or `unsure`) and returned to the frontend.

## Testing Locally
To test the backend locally, you can use the Vercel CLI:
```bash
vercel dev
```
Make sure you have a `.env` file with your Sightengine credentials.

## License
MIT
