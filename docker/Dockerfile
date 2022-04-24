FROM  amazonlinux:latest
EXPOSE 8080/tcp
EXPOSE 1935/tcp
EXPOSE 4040/tcp

VOLUME [ "/etc/localtime" ]

RUN yum groupinstall -y "Development Tools" pcre2 pcre2-devel openssl-devel

# install noip client
RUN cd /usr/local/src && wget http://www.noip.com/client/linux/noip-duc-linux.tar.gz \
    && tar -xvf *.tar.gz && cd noip* && make && cp noip2 /usr/local/bin
COPY no-ip2.conf usr/local/etc/no-ip2.conf

# install nginx
RUN cd /tmp && git clone https://github.com/sergey-dryabzhinsky/nginx-rtmp-module.git \
    && wget https://nginx.org/download/nginx-1.21.6.tar.gz \
    && tar -xf nginx-1.21.6.tar.gz && cd nginx-1.21.6 && \
    ./configure --with-http_ssl_module --add-module=../nginx-rtmp-module &&\
    make -j 2 && make install
RUN chown -R ec2-user:ec2-user /usr/local/nginx && mkdir /srv/nginx && \
    chmod 755 -R /srv/nginx && export PATH=/usr/local/nginx/sbin/:$PATH
COPY nginx.conf /usr/local/nginx/conf/nginx.conf

CMD noip2 && nginx