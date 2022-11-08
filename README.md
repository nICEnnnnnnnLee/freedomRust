<h1 align="center">  
    <strong>
        Freedom
    </strong>
</h1>
<p align="center">
    端到端数据流量伪装加密研究
  <br/>
    <strong>仅供学习研究使用，请勿用于非法用途</strong>
</p>


## :star:相关Repo
| 项目名称  | 简介 | 
| ------------- | ------------- |   
| [freedomRust](https://github.com/nICEnnnnnnnLee/freedomRust)  |  Rust实现，包含local端、remote端  | 
| [freedomGo](https://github.com/nICEnnnnnnnLee/freedomGo)  |  Go实现，包含local端、remote端  | 
| [freedom4py](https://github.com/nICEnnnnnnnLee/freedom4py)  |  python3实现，包含local端、remote端  | 
| [freedom4j](https://github.com/nICEnnnnnnnLee/freedom4j)  |  java实现，包含local端、remote端  | 
| [freedom4NG](https://github.com/nICEnnnnnnnLee/freedom4NG)  | Android java实现，仅包含local端；单独使用可作为DNS、Host修改器 | 
 



## :star:一句话说明  
将本地代理数据伪装成指向远程端的HTTP(S) WebSocket流量。

## :star:简介  
+ 在配置正确的情况下，Rust、Go、python3、java、Android版本的local端和remote端可以配合使用。  
+ local端实现了HTTP(S)代理。  
+ local端到remote端可以套上一层HTTP(S)，表现行为与Websocket无异，经测试**可过CDN与Nginx**。  
+ local端到remote端支持简单的用户名密码验证。  

## :star:缺陷  
+ 仅支持TCP，不支持UDP
+ Rust版本: 因为从local到remote的连接，如果不加密的话毫无意义，所以local端没有不使用加密连接的选项  
+ Rust版本: (相较于Go版本)local端没有实现根据域名或IP分流
+ Rust版本: (相较于Go版本)local端没有实现Socks5代理

## :star:如何配置  


<details>
<summary>local端配置</summary>



```yml

bind_host: 127.0.0.1 
bind_port: 1081
# 该值可以是ip或者域名
remote_host: 127.0.0.1 
remote_port: 443
username: username
password: pwd
salt: 1234abc
# 是否允许不安全的HTTPS连接
allow_insecure: false
http_path: /
# 该值还是加密连接Client Hello消息里面的SNI
http_domain: baidu.com
http_user_agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:86.0) Gecko/20100101 Firefox/86.0
```
</details>

<details>
<summary>remote端配置</summary>



```yml
bind_host: 127.0.0.1
bind_port: 443 
salt: "4567"

use_ssl: false
# 当use_ssl为false时，下面三个选项可以随便填写(但不能没有)
sni: baidu.com
cert_path: data/fullchain.pem
key_path: data/www.baidu.com.key
users:
  user1: pwd1 
  username: pwd
  
```
</details>








## :star:如何运行  
+ 运行本地端  
```
freedom-rust local "配置文件路径"
```

+ 运行远程端
```
freedom-rust remote "配置文件路径"
```