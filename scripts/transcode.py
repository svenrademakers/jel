import subprocess
import os

input_device=" -i video=\"Game Capture HD60 S+\":audio=\"Digital Audio Interface (Game Capture HD60 S+)\""
stream_name="sjaak"
cwd="Z:\\"

cmd= f'ffmpeg -f dshow -rtbufsize 1024M  {input_device} -r 30   \
-filter_complex \
"[0:v]split=3[v1][v2][v3]; \
[v1]copy[v1out]; [v2]scale=w=1280:h=720[v2out]; [v3]scale=w=640:h=360[v3out]" \
-map [v1out] -c:v:0 libx264 -x264-params "nal-hrd=cbr:force-cfr=1" -b:v:0 14M -maxrate:v:0 14M -minrate:v:0 12M -bufsize:v:0 14M -sc_threshold 0 -keyint_min 48 \
-map [v2out] -c:v:1 libx264 -x264-params "nal-hrd=cbr:force-cfr=1" -b:v:1 6M -maxrate:v:1 6M -minrate:v:1 4M -bufsize:v:1 6M -sc_threshold 0 -keyint_min 48 \
-map [v3out] -c:v:2 libx264 -x264-params "nal-hrd=cbr:force-cfr=1" -b:v:2 3M -maxrate:v:2 3M -minrate:v:2 2M -bufsize:v:2 3M  -sc_threshold 0 -keyint_min 48 \
-map a:0 -c:a:0 aac -b:a:0 128k -ac 2 \
-map a:0 -c:a:1 aac -b:a:1 96k -ac 2 \
-map a:0 -c:a:2 aac -b:a:2 48k -ac 2 \
-f hls \
-hls_time 2 \
-hls_list_size 2 \
-hls_flags independent_segments \
-hls_segment_type mpegts \
-hls_segment_filename .\{stream_name}\%v_%02d.ts \
-master_pl_name .\{stream_name}_vod.m3u8 \
-var_stream_map "v:0,a:0 v:1,a:1 v:2,a:2" .\{stream_name}\%v.m3u8 '
print(cmd)
subprocess.run(cmd, shell=True, cwd=cwd)