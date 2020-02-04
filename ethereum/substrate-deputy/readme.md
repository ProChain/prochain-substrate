# Deputy代理

## 启动

路径 `/data/mainnet/prochain-substrate/ethereum/substrate-deputy`

命令 `node index.js`

## 守护进程supervisor

生产环务必开启守进程

### 安装

`yum install supervisor`

#### 设置开机自启

`systemctl enable supervisord.service`

#### 配置文件

位置 `/etc/supervisord.d/deputy.conf`

若不存在，则新建一个 `echo_supervisord_conf > /etc/supervisord.d/deputy.conf`

```
[unix_http_server]
file=/var/run/supervisor.sock   ; (the path to the socket file)

[rpcinterface:supervisor]
supervisor.rpcinterface_factory = supervisor.rpcinterface:make_main_rpcinterface

[supervisorctl]
serverurl=unix:///var/run/supervisor.sock ;

[supervisord]
logfile=/tmp/supervisord.log ; (main log file;default $CWD/supervisord.log)
logfile_maxbytes=50MB        ; (max main logfile bytes b4 rotation;default 50MB)
logfile_backups=10           ; (num of main logfile rotation backups;default 10)
loglevel=info                ; (log level;default info; others: debug,warn,trace)
pidfile=/tmp/supervisord.pid ; (supervisord pidfile;default supervisord.pid)
nodaemon=false               ; (start in foreground if true;default false)
minfds=1024                  ; (min. avail startup file descriptors;default 1024)
minprocs=200                 ; (min. avail process descriptors;default 200)

[program:deputy]
command=node index.js              ; the program (relative uses PATH, can take args)
;process_name=%(program_name)s ; process_name expr (default %(program_name)s)
numprocs=1                    ; number of processes copies to start (def 1)
directory=/data/mainnet/prochain-substrate/ethereum/substrate-deputy   ; directory to cwd to before exec (def no cwd)
autostart=true                ; start at supervisord start (default: true)
autorestart=unexpected        ; whether/when to restart (default: unexpected)
startsecs=1                   ; number of secs prog must stay running (def. 1)
startretries=3                ; max # of serial start failures (default 3)
stdout_logfile=/data/mainnet/prochain-substrate/ethereum/substrate-deputy/deputy.info.log        ; stdout log path, NONE for none; default AUTO
stdout_logfile_maxbytes=100MB   ; max # logfile bytes b4 rotation (default 50MB)
stdout_logfile_backups=10     ; # of stdout logfile backups (default 10)
;stdout_capture_maxbytes=1MB   ; number of bytes in 'capturemode' (default 0)
;stdout_events_enabled=false   ; emit events on stdout writes (default false)
stderr_logfile=/data/mainnet/prochain-substrate/ethereum/substrate-deputy/deputy.err.log        ; stderr log path, NONE for none; default AUTO
stderr_logfile_maxbytes=100MB   ; max # logfile bytes b4 rotation (default 50MB)
stderr_logfile_backups=10     ; # of stderr logfile backups (default 10)
```

#### 日志文件

```
/data/mainnet/prochain-substrate/ethereum/substrate-deputy/deputy.info.log
/data/mainnet/prochain-substrate/ethereum/substrate-deputy/deputy.err.log
/tmp/supervisord.log
```

### 启动supervisord
`supervisord -c /etc/supervisord.d/deputy.conf`

### 管理指定进程

```
#查看监听了哪些进程
supervisorctl status

#重启指定进程
supervisorctl start deputy

#重启所有进程
supervisorctl start all

#停止指定进程
supervisorctl stop deputy

#停止所有进程
supervisorctl stop all
```

