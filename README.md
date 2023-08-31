# fpp-media-server

This is a proof-of-concept for allowing people watching a display run with [FPP](https://github.com/FalconChristmas/fpp) to tune into the music on their phone, rather than using a radio.

It is very much in the proof-of-concept stage, so far only used for a few demos, and targetting use on my own display for the upcoming year.

That being said, if you want to use it, just put all the media files from FPP into the `audio` folder in this repo (with the same filenames) and run the server with `cargo`. Then, install the [accompanying plugin](https://github.com/jfrydell/fpp-Sync) on your FPP, setting the POST endpoint accordingly (right now, this can only be configured in the plugin source). Connecting your phone to the server of HTTP should show a basic UI with a "Start" button, which can be clicked to start and synchronize the audio. If the audio goes out of sync (for example, due to closing/reopening your browser), just push the "Resync" button.
