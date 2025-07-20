# Monolith Docker Deployment Guide

本指南介绍如何使用Docker部署Monolith网页翻译工具。

## 🚀 快速开始

### 方式一：使用快速启动脚本

```bash
# 克隆项目并进入目录
cd monolith/

# 运行快速启动脚本
./start.sh
```

启动后访问：
- 🌐 主界面: http://localhost:7080
- 📚 翻译库: http://localhost:7080/library
- 🔧 Redis管理: http://localhost:8081 (用户名/密码: admin/secret)

### 方式二：使用Docker Compose

```bash
# 启动开发环境
docker-compose up -d

# 查看服务状态
docker-compose ps

# 查看日志
docker-compose logs -f
```

### 方式三：使用Makefile

```bash
# 查看所有可用命令
make -f Makefile.docker help

# 启动开发环境
make -f Makefile.docker dev

# 启动生产环境
make -f Makefile.docker prod
```

## 📁 项目结构

```
monolith/
├── Dockerfile.web              # Web应用容器配置
├── docker-compose.yml          # 开发环境配置
├── docker-compose.prod.yml     # 生产环境配置
├── deploy.sh                   # 部署脚本
├── start.sh                    # 快速启动脚本
├── Makefile.docker             # Docker操作Makefile
├── .env.example                # 环境变量示例
├── .env.prod.example           # 生产环境变量示例
└── config/                     # 配置文件目录
    ├── redis.conf              # Redis配置
    ├── redis.prod.conf          # Redis生产配置
    └── nginx.conf               # Nginx配置(生产环境)
```

## 🛠️ 环境配置

### 开发环境

1. 复制环境变量文件：
```bash
cp .env.example .env
```

2. 修改配置（可选）：
```bash
# 编辑 .env 文件，调整以下配置
RUST_LOG=info                   # 日志级别
PORT=7080                       # 端口号
REDIS_URL=redis://redis:6379    # Redis连接地址
```

3. 复制翻译配置：
```bash
cp translation-config.toml.example translation-config.toml
```

### 生产环境

1. 复制生产环境配置：
```bash
cp .env.prod.example .env.prod
```

2. 修改生产配置：
```bash
# 编辑 .env.prod 文件
RUST_LOG=warn                           # 生产环境日志级别
REDIS_PASSWORD=your-secure-password     # Redis密码
JWT_SECRET=your-jwt-secret              # JWT密钥
CORS_ORIGIN=https://your-domain.com     # CORS域名
```

3. 配置SSL证书（如需要）：
```bash
# 将SSL证书放入config/ssl目录
mkdir -p config/ssl
cp your-cert.pem config/ssl/cert.pem
cp your-key.pem config/ssl/key.pem
```

## 🔧 服务管理

### 使用部署脚本

```bash
# 启动开发环境
./deploy.sh dev start

# 启动生产环境
./deploy.sh prod start

# 停止服务
./deploy.sh dev stop

# 重启服务
./deploy.sh dev restart

# 查看日志
./deploy.sh dev logs

# 查看状态
./deploy.sh dev status
```

### 使用Docker Compose

```bash
# 开发环境
docker-compose up -d              # 启动
docker-compose down               # 停止
docker-compose logs -f            # 查看日志
docker-compose ps                 # 查看状态

# 生产环境
docker-compose -f docker-compose.prod.yml up -d
docker-compose -f docker-compose.prod.yml down
```

## 📊 监控和维护

### 查看服务状态

```bash
# 使用脚本
./deploy.sh dev status

# 使用Docker命令
docker-compose ps
docker stats --no-stream
```

### 查看日志

```bash
# 所有服务日志
docker-compose logs -f

# 特定服务日志
docker-compose logs -f monolith-web
docker-compose logs -f redis

# 实时查看最新日志
docker-compose logs -f --tail=100 monolith-web
```

### Redis管理

```bash
# 进入Redis CLI
docker-compose exec redis redis-cli

# 查看Redis信息
docker-compose exec redis redis-cli info

# 查看缓存统计
docker-compose exec redis redis-cli info keyspace
```

### 数据备份

```bash
# 备份Redis数据
make -f Makefile.docker backup

# 恢复Redis数据
make -f Makefile.docker restore BACKUP_FILE=backups/backup-20231201-120000.rdb
```

## 🔒 安全配置

### 生产环境安全建议

1. **更改默认密码**：
   - Redis密码
   - Redis Commander密码
   - JWT密钥

2. **配置SSL**：
   - 使用HTTPS
   - 配置SSL证书
   - 启用安全头

3. **网络安全**：
   - 仅暴露必要端口
   - 使用防火墙
   - 配置反向代理

4. **定期更新**：
   - 更新Docker镜像
   - 更新依赖包
   - 监控安全漏洞

## 🐛 故障排除

### 常见问题

1. **端口冲突**：
```bash
# 检查端口占用
netstat -tlnp | grep :7080

# 修改端口配置
# 编辑 .env 文件中的 PORT 变量
```

2. **Redis连接失败**：
```bash
# 检查Redis服务状态
docker-compose exec redis redis-cli ping

# 重启Redis服务
docker-compose restart redis
```

3. **内存不足**：
```bash
# 查看内存使用
docker stats

# 增加系统内存限制
# 编辑 docker-compose.yml 中的 deploy.resources 配置
```

4. **构建失败**：
```bash
# 清理Docker缓存
docker system prune -f

# 重新构建镜像
docker-compose build --no-cache
```

### 日志分析

```bash
# 查看容器日志
docker-compose logs monolith-web | grep ERROR

# 查看系统资源
docker stats --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}"

# 检查容器健康状态
docker-compose ps
```

## 📈 性能优化

### 资源配置

1. **内存限制**：
```yaml
# 在 docker-compose.yml 中配置
deploy:
  resources:
    limits:
      memory: 1G
    reservations:
      memory: 512M
```

2. **CPU限制**：
```yaml
deploy:
  resources:
    limits:
      cpus: '1.0'
```

3. **Redis优化**：
```conf
# config/redis.conf
maxmemory 512mb
maxmemory-policy allkeys-lru
```

### 缓存优化

- 永久缓存模式减少重复翻译
- 定期清理无用缓存
- 监控缓存命中率

## 🔄 更新部署

```bash
# 拉取最新代码
git pull

# 重新构建并部署
docker-compose down
docker-compose build --no-cache
docker-compose up -d

# 或使用Makefile
make -f Makefile.docker rebuild
```

## 📞 支持

如果遇到问题，请：

1. 查看日志文件
2. 检查配置文件
3. 参考故障排除章节
4. 提交Issue到GitHub仓库

---

**注意**: 生产环境部署前请仔细阅读安全配置章节，确保系统安全。