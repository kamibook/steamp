## 反向代理工具

### 前言

   解决受到sni污染的问题，例如steam相关域名；使用rust开发，具有高性能，极低的CPU 内存占用相较于其它语言。
   核心功能与steamcommunity302相同。

### 运行

```
./steamp #命令行运行
./steamp -d #后台运行，重启系统会停止
```

### 配置 config.toml

- 可执行文件会从可执行文件所在目录加载配置文件，例如: 可执行文件路径`/root/steamp`，则会加载`/root/config.toml`。
  
```
https = "0.0.0.0:443"                                 # https 监听地址(很重要，一定要设置)
sni = "steamuserimages-a.akamaihd.net.edgesuite.net"  # 向上游发起请求时提交的SNI(无所谓，可随意设置大陆可访问的域名)
cert = "steamcommunity.crt"                           # ssl证书，绝对路径
key = "steamcommunity.key"                            # ssl秘钥，绝对路径

# 负载均衡上游地址，为steam相关域名解析出的CDN IP，可设置多个，默认开启健康检查(频率1s/次)，当某IP不可访问，请求将会规避该IP至通过健康检查，默认KetamaHashing策略即每个客户端的所有请求只会发送到初始后端
backends = [
    "23.51.204.111:443",
    "23.51.204.1122:443"
]
```

### 自签证书

```
https://github.com/kamibook/steamopenssl 

./steamopenssl  #生成CA自签证书，已默认设置steam相关域名，有效期10年
```