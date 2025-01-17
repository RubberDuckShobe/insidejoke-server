# [Inside Joke](https://www.youtube.com/watch?v=ZUwaudw8ht0) (server)

Bad code that does horrible, absolutely no good things.

## How does it work?
client mod -> server application -> Whisper (locally running. please don't send others' voice data to external services.) -> text

> [!WARNING]  
> Whisper tends to hallucinate and add sentences on short audio since it's made to operate on 30 second long chunks of audio.
> This results in additional, YouTube-esque sentences being added to Whisper's output.

Also, please run this on a machine in your local network. It can be run on the same machine as the client and when running on a GPU, shouldn't take up that much VRAM even with the `ggml-large-v3` model. I use an RTX 2070, which runs Resonite twice and Whisper with the large-v3 mode, all at the same time and without issues.

Nothing is stopping you from hosting this on some server on the Internet, but please be aware that:
1. you'd be constantly streaming raw audio data over the internet which might be heavy.
2. you'd be constantly streaming other people's voices to a machine controlled by another company. Please don't do that.

It might be worth exploring alternate audio recognition models or software outside of Whisper that are better suited to streaming audio.