ffmpeg -re -f dshow -rtbufsize 100M -i video="Game Capture HD60 S+" -vcodec libx264 -acodec aac -b:v 6000K -b:a 192k -f  flv rtmp://live.svenrademakers.com:1935/show/live
