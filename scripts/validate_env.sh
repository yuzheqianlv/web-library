#!/bin/bash

# ============================================
# Monolith 环境变量配置验证脚本
# ============================================
# 
# 此脚本用于验证环境变量配置的正确性
# 使用方法: ./scripts/validate_env.sh [环境名称]
# 示例: ./scripts/validate_env.sh development

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 脚本参数
ENVIRONMENT=${1:-"development"}
ENV_FILE=".env.${ENVIRONMENT}"

# 如果指定环境文件不存在，尝试使用 .env
if [[ ! -f "$ENV_FILE" ]]; then
    if [[ -f ".env" ]]; then
        ENV_FILE=".env"
        echo -e "${YELLOW}警告: ${ENV_FILE} 不存在，使用 .env 文件${NC}"
    else
        echo -e "${RED}错误: 找不到环境配置文件 ${ENV_FILE} 或 .env${NC}"
        exit 1
    fi
fi

echo -e "${BLUE}=== Monolith 环境变量配置验证 ===${NC}"
echo -e "环境: ${ENVIRONMENT}"
echo -e "配置文件: ${ENV_FILE}"
echo ""

# 加载环境变量
source "$ENV_FILE"

# 验证计数器
TOTAL_CHECKS=0
PASSED_CHECKS=0
WARNINGS=0

# 检查函数
check_var() {
    local var_name="$1"
    local description="$2"
    local is_required="${3:-false}"
    local validation_func="${4:-}"
    
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    if [[ -z "${!var_name}" ]]; then
        if [[ "$is_required" == "true" ]]; then
            echo -e "${RED}✗ $var_name${NC} - $description [必需变量未设置]"
            return 1
        else
            echo -e "${YELLOW}⚠ $var_name${NC} - $description [可选变量未设置，将使用默认值]"
            WARNINGS=$((WARNINGS + 1))
            return 0
        fi
    fi
    
    # 如果有验证函数，执行验证
    if [[ -n "$validation_func" ]]; then
        if $validation_func "${!var_name}"; then
            echo -e "${GREEN}✓ $var_name${NC} - $description [${!var_name}]"
            PASSED_CHECKS=$((PASSED_CHECKS + 1))
            return 0
        else
            echo -e "${RED}✗ $var_name${NC} - $description [验证失败: ${!var_name}]"
            return 1
        fi
    else
        echo -e "${GREEN}✓ $var_name${NC} - $description [${!var_name}]"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
        return 0
    fi
}

# 验证函数
validate_mode() {
    case "$1" in
        development|staging|production) return 0 ;;
        *) echo "无效模式，应为: development, staging, production"; return 1 ;;
    esac
}

validate_log_level() {
    case "$1" in
        trace|debug|info|warn|error) return 0 ;;
        *) echo "无效日志级别，应为: trace, debug, info, warn, error"; return 1 ;;
    esac
}

validate_boolean() {
    case "${1,,}" in
        true|false|1|0|yes|no|on|off|enabled|disabled) return 0 ;;
        *) echo "无效布尔值，应为: true/false, 1/0, yes/no, on/off, enabled/disabled"; return 1 ;;
    esac
}

validate_port() {
    if [[ "$1" =~ ^[0-9]+$ ]] && [ "$1" -ge 1 ] && [ "$1" -le 65535 ]; then
        return 0
    else
        echo "无效端口号，应为 1-65535 之间的数字"
        return 1
    fi
}

validate_url() {
    if [[ "$1" =~ ^https?://[^[:space:]]+$ ]]; then
        return 0
    else
        echo "无效URL格式，应以 http:// 或 https:// 开头"
        return 1
    fi
}

validate_lang_code() {
    if [[ "$1" == "auto" ]] || [[ "$1" =~ ^[a-z]{2}$ ]]; then
        return 0
    else
        echo "无效语言代码，应为 'auto' 或2位ISO 639-1代码"
        return 1
    fi
}

validate_positive_number() {
    if [[ "$1" =~ ^[0-9]+$ ]] && [ "$1" -gt 0 ]; then
        return 0
    else
        echo "应为正整数"
        return 1
    fi
}

validate_positive_float() {
    if [[ "$1" =~ ^[0-9]+\.?[0-9]*$ ]] && [[ "$1" != "0" ]] && [[ "$1" != "0.0" ]]; then
        return 0
    else
        echo "应为正数"
        return 1
    fi
}

validate_mongodb_url() {
    if [[ "$1" =~ ^mongodb(\+srv)?://[^[:space:]]+$ ]]; then
        return 0
    else
        echo "无效MongoDB URL，应以 mongodb:// 或 mongodb+srv:// 开头"
        return 1
    fi
}

echo -e "${BLUE}核心配置验证:${NC}"
check_var "MONOLITH_MODE" "应用运行模式" false "validate_mode"
check_var "MONOLITH_LOG_LEVEL" "日志级别" false "validate_log_level"
check_var "NO_COLOR" "禁用彩色输出" false "validate_boolean"

echo -e "\n${BLUE}翻译配置验证:${NC}"
check_var "MONOLITH_TRANSLATION_ENABLED" "翻译功能开关" false "validate_boolean"
check_var "MONOLITH_TRANSLATION_TARGET_LANG" "目标语言" false "validate_lang_code"
check_var "MONOLITH_TRANSLATION_SOURCE_LANG" "源语言" false "validate_lang_code"
check_var "MONOLITH_TRANSLATION_API_URL" "翻译API URL" false "validate_url"
check_var "MONOLITH_TRANSLATION_MAX_REQUESTS_PER_SECOND" "每秒最大请求数" false "validate_positive_float"
check_var "MONOLITH_TRANSLATION_MAX_CONCURRENT_REQUESTS" "最大并发请求数" false "validate_positive_number"
check_var "MONOLITH_TRANSLATION_BATCH_SIZE" "批次大小" false "validate_positive_number"
check_var "MONOLITH_TRANSLATION_BATCH_TIMEOUT" "批次超时时间" false "validate_positive_number"

echo -e "\n${BLUE}缓存配置验证:${NC}"
check_var "MONOLITH_CACHE_ENABLED" "缓存功能开关" false "validate_boolean"
check_var "MONOLITH_CACHE_LOCAL_SIZE" "本地缓存大小" false "validate_positive_number"
check_var "MONOLITH_CACHE_TTL" "缓存TTL" false "validate_positive_number"
check_var "MONOLITH_CACHE_WARMUP_ENABLED" "缓存预热" false "validate_boolean"

echo -e "\n${BLUE}Web服务器配置验证:${NC}"
check_var "MONOLITH_WEB_BIND_ADDRESS" "绑定地址" false
check_var "MONOLITH_WEB_PORT" "监听端口" false "validate_port"
check_var "MONOLITH_WEB_STATIC_DIR" "静态文件目录" false

echo -e "\n${BLUE}MongoDB配置验证:${NC}"
check_var "MONGODB_URL" "MongoDB连接字符串" false "validate_mongodb_url"
check_var "MONGODB_DATABASE" "数据库名称" false
check_var "MONGODB_COLLECTION" "集合名称" false

echo -e "\n${BLUE}性能配置验证:${NC}"
check_var "MONOLITH_WORKER_THREADS" "工作线程数" false "validate_positive_number"
check_var "MONOLITH_MAX_MEMORY_MB" "最大内存限制" false "validate_positive_number"
check_var "MONOLITH_PARALLEL_ENABLED" "并行处理" false "validate_boolean"

echo -e "\n${BLUE}安全配置验证:${NC}"
check_var "MONOLITH_API_KEY" "API密钥" false
check_var "MONOLITH_CORS_ORIGINS" "CORS允许源" false

# 高级验证
echo -e "\n${BLUE}高级验证:${NC}"

# 检查翻译API连接
if [[ -n "$MONOLITH_TRANSLATION_API_URL" ]]; then
    echo -n "测试翻译API连接... "
    if curl -s --connect-timeout 5 --max-time 10 "$MONOLITH_TRANSLATION_API_URL" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ API可访问${NC}"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
    else
        echo -e "${YELLOW}⚠ API不可访问${NC} (可能服务未启动)"
        WARNINGS=$((WARNINGS + 1))
    fi
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
fi

# 检查MongoDB连接
if [[ -n "$MONGODB_URL" ]] && command -v mongosh >/dev/null 2>&1; then
    echo -n "测试MongoDB连接... "
    if timeout 5 mongosh "$MONGODB_URL" --eval "db.runCommand('ping')" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ MongoDB可连接${NC}"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
    else
        echo -e "${YELLOW}⚠ MongoDB不可连接${NC} (可能服务未启动)"
        WARNINGS=$((WARNINGS + 1))
    fi
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
elif [[ -n "$MONGODB_URL" ]]; then
    echo -e "${YELLOW}⚠ mongosh未安装，跳过MongoDB连接测试${NC}"
    WARNINGS=$((WARNINGS + 1))
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
fi

# 检查端口是否可用
if [[ -n "$MONOLITH_WEB_PORT" ]]; then
    echo -n "检查端口 $MONOLITH_WEB_PORT 可用性... "
    if ! lsof -i ":$MONOLITH_WEB_PORT" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ 端口可用${NC}"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
    else
        echo -e "${YELLOW}⚠ 端口已被占用${NC}"
        WARNINGS=$((WARNINGS + 1))
    fi
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
fi

# 验证静态文件目录
if [[ -n "$MONOLITH_WEB_STATIC_DIR" ]]; then
    echo -n "检查静态文件目录... "
    if [[ -d "$MONOLITH_WEB_STATIC_DIR" ]]; then
        echo -e "${GREEN}✓ 目录存在${NC}"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
    else
        echo -e "${YELLOW}⚠ 目录不存在: $MONOLITH_WEB_STATIC_DIR${NC}"
        WARNINGS=$((WARNINGS + 1))
    fi
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
fi

# 输出总结
echo -e "\n${BLUE}=== 验证总结 ===${NC}"
echo -e "总检查项: $TOTAL_CHECKS"
echo -e "通过检查: ${GREEN}$PASSED_CHECKS${NC}"
echo -e "警告信息: ${YELLOW}$WARNINGS${NC}"
echo -e "失败检查: ${RED}$((TOTAL_CHECKS - PASSED_CHECKS - WARNINGS))${NC}"

# 配置建议
echo -e "\n${BLUE}配置建议:${NC}"
if [[ "$ENVIRONMENT" == "development" ]]; then
    echo "• 开发环境建议启用debug日志级别以便调试"
    echo "• 可以使用较小的缓存和并发设置减少资源占用"
    echo "• 建议使用本地翻译API服务"
elif [[ "$ENVIRONMENT" == "production" ]]; then
    echo "• 生产环境建议使用warn或error日志级别"
    echo "• 确保设置了API密钥以提高安全性"
    echo "• 建议启用缓存以提高性能"
    echo "• 考虑配置负载均衡和监控"
fi

# 退出状态
FAILED_CHECKS=$((TOTAL_CHECKS - PASSED_CHECKS - WARNINGS))
if [[ $FAILED_CHECKS -gt 0 ]]; then
    echo -e "\n${RED}存在配置错误，请修复后重新验证${NC}"
    exit 1
elif [[ $WARNINGS -gt 0 ]]; then
    echo -e "\n${YELLOW}配置基本正确，但存在警告项${NC}"
    exit 0
else
    echo -e "\n${GREEN}✓ 所有配置验证通过！${NC}"
    exit 0
fi