Download queue row (Steam's Downloads page). The stage label is mono and says the step the install is at: Queued, Downloading, Verifying, Extracting, Finishing (the release's file is downloaded, checked against its SHA-256, unpacked, and moved over the app's folder). Freya: `ProgressBar` inside a flex column; the progress fill is accent (info), switching to ok at 100% verified and danger on failure; failure always shows the reason in the detail line, never just a red bar.

Cancel is an icon ghost button with an accessible label.
