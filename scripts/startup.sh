#!/bin/sh
yum groupinstall "Development Tools"
cd /tmp/
wget http://www.noip.com/client/linux/noip-duc-linux.tar.gz
tar -xvf *.tar.gz
cd noip*
make install

### install nginx + rtmp plugin
yum -y install pcre2 pcre2-devel openssl-devel
git clone https://github.com/sergey-dryabzhinsky/nginx-rtmp-module.git
wget https://nginx.org/download/nginx-1.21.6.tar.gz
tar -xf nginx-1.21.6.tar.gz
cd nginx-1.21.6
./configure --with-http_ssl_module --add-module=../nginx-rtmp-module
make -j 1
sudo make install

tee  /usr/local/nginx/conf/nginx.conf << END
worker_processes  auto;
events {
    worker_connections  1024;
}

error_log /var/log/nginx_error.log info;

# RTMP configuration
rtmp {
    server {
        listen 1935; # Listen on standard RTMP port
        chunk_size 4000;

        application show {
            live on;
            hls on;
            hls_path /mnt/hls/;

            dash on;
            dash_path /mnt/dash;
            # disable consuming the stream from nginx as rtmp
            deny play all;
        }
    }
}

http {
    sendfile off;
    tcp_nopush on;
    directio 512;
    default_type application/octet-stream;

    server {
        listen 8080;

        location / {
            # Disable cache
            add_header 'Cache-Control' 'no-cache';

            # CORS setup
            add_header 'Access-Control-Allow-Origin' '*' always;
            add_header 'Access-Control-Expose-Headers' 'Content-Length';

            # allow CORS preflight requests
            if (\$request_method = 'OPTIONS') {
                add_header 'Access-Control-Allow-Origin' '*';
                add_header 'Access-Control-Max-Age' 1728000;
                add_header 'Content-Type' 'text/plain charset=UTF-8';
                add_header 'Content-Length' 0;
                return 204;
            }

            types {
                application/dash+xml mpd;
                application/vnd.apple.mpegurl m3u8;
                video/mp2t ts;
            }

            root /mnt/;
        }
    }
}
END
chown -R ec2-user:ec2-user /usr/local/nginx /mnt
chmod -R 777 /mnt
export PATH=/usr/local/nginx/sbin/:$PATH

/usr/local/nginx/sbin/nginx
noip2
