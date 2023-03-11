ffmpeg -t 8400 -f dshow -rtbufsize 2048M -i video="Game Capture HD60 S+":audio="Digital Audio Interface (Game Capture HD60 S+)"^
    -r 30 -pix_fmt yuv420p^
    -c:v libx264 -preset -profile:v high -level:v 4.1 -b:v 14M^
    -c:a aac -b:a 192k %1
    