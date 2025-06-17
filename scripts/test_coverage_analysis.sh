#!/bin/bash

# 🔍 matrixon Matrix Server - 测试覆盖分析脚本
# 
# Author: matrixon Team  
# Date: 2024-12-19
# Version: 1.0.0
# Purpose: 分析项目测试覆盖情况，识别需要补充测试的模块

echo "🔍 matrixon Matrix Server - 测试覆盖分析报告"
echo "============================================"
echo ""

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 统计变量
TOTAL_FILES=0
FILES_WITH_TESTS=0
FILES_WITHOUT_TESTS=0
API_FILES_WITHOUT_TESTS=0
SERVICE_FILES_WITHOUT_TESTS=0
CLI_FILES_WITHOUT_TESTS=0

# 辅助函数
log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

echo "📊 第1步: 项目测试覆盖统计"
echo "------------------------"

# 统计总文件数
TOTAL_FILES=$(find src/ -name "*.rs" | wc -l | tr -d ' ')
log_info "项目总Rust文件数: $TOTAL_FILES"

# 统计有测试的文件数
FILES_WITH_TESTS=$(find src/ -name "*.rs" -exec grep -l "#\[cfg(test)\]" {} \; | wc -l | tr -d ' ')
log_success "包含测试的文件数: $FILES_WITH_TESTS"

# 计算无测试的文件数
FILES_WITHOUT_TESTS=$((TOTAL_FILES - FILES_WITH_TESTS))
log_warning "缺少测试的文件数: $FILES_WITHOUT_TESTS"

# 计算覆盖率
COVERAGE_RATE=$((FILES_WITH_TESTS * 100 / TOTAL_FILES))
echo -e "${BLUE}当前测试覆盖率: ${YELLOW}$COVERAGE_RATE%${NC}"

echo ""
echo "🔍 第2步: 按模块分析缺失测试"
echo "--------------------------"

# 创建临时文件存储分析结果
TEMP_FILE="/tmp/missing_tests.txt"
find src/ -name "*.rs" -exec grep -L "#\[cfg(test)\]" {} \; > "$TEMP_FILE"

# 按目录分类分析
echo "## API模块缺失测试:"
API_FILES=$(grep "src/api/" "$TEMP_FILE" | wc -l | tr -d ' ')
API_FILES_WITHOUT_TESTS=$API_FILES
echo "缺失测试的API文件数: $API_FILES"
grep "src/api/" "$TEMP_FILE" | head -10 | sed 's/^/  - /'
if [ $API_FILES -gt 10 ]; then
    echo "  ... 以及其他 $((API_FILES - 10)) 个文件"
fi

echo ""
echo "## Service模块缺失测试:"
SERVICE_FILES=$(grep "src/service/" "$TEMP_FILE" | wc -l | tr -d ' ')
SERVICE_FILES_WITHOUT_TESTS=$SERVICE_FILES
echo "缺失测试的Service文件数: $SERVICE_FILES"
grep "src/service/" "$TEMP_FILE" | head -10 | sed 's/^/  - /'
if [ $SERVICE_FILES -gt 10 ]; then
    echo "  ... 以及其他 $((SERVICE_FILES - 10)) 个文件"
fi

echo ""
echo "## CLI模块缺失测试:"
CLI_FILES=$(grep "src/cli/" "$TEMP_FILE" | wc -l | tr -d ' ')
CLI_FILES_WITHOUT_TESTS=$CLI_FILES
echo "缺失测试的CLI文件数: $CLI_FILES"
grep "src/cli/" "$TEMP_FILE" | head -10 | sed 's/^/  - /'

echo ""
echo "## 数据库模块缺失测试:"
DB_FILES=$(grep "src/database/" "$TEMP_FILE" | wc -l | tr -d ' ')
echo "缺失测试的数据库文件数: $DB_FILES"
grep "src/database/" "$TEMP_FILE" | head -5 | sed 's/^/  - /'

echo ""
echo "📋 第3步: 优先级分析"
echo "------------------"

echo "🔥 高优先级 (API和核心服务):"
grep -E "src/(api|service)/" "$TEMP_FILE" | grep -E "(mod\.rs|server\.rs|client\.rs|auth\.rs|room\.rs)" | sed 's/^/  - /'

echo ""
echo "🔧 中优先级 (功能模块):"
grep -E "src/(api|service)/" "$TEMP_FILE" | grep -vE "(mod\.rs|server\.rs|client\.rs|auth\.rs|room\.rs)" | head -15 | sed 's/^/  - /'

echo ""
echo "⚡ 低优先级 (CLI和工具):"
grep -E "src/(cli|utils|test_utils)" "$TEMP_FILE" | head -10 | sed 's/^/  - /'

echo ""
echo "📈 第4步: 测试质量分析"
echo "--------------------"

# 分析现有测试的质量
echo "## 现有测试函数统计:"
TEST_FUNCTIONS=$(find src/ -name "*.rs" -exec grep -h "#\[test\]" {} \; | wc -l | tr -d ' ')
ASYNC_TEST_FUNCTIONS=$(find src/ -name "*.rs" -exec grep -h "#\[tokio::test\]" {} \; | wc -l | tr -d ' ')
IGNORED_TESTS=$(find src/ -name "*.rs" -exec grep -h "#\[ignore\]" {} \; | wc -l | tr -d ' ')

echo "  - 同步测试函数: $TEST_FUNCTIONS"
echo "  - 异步测试函数: $ASYNC_TEST_FUNCTIONS"
echo "  - 被忽略的测试: $IGNORED_TESTS"
echo "  - 总测试函数: $((TEST_FUNCTIONS + ASYNC_TEST_FUNCTIONS))"

echo ""
echo "## 测试模式分析:"
UNIT_TESTS=$(find src/ -name "*.rs" -exec grep -l "fn test_" {} \; | wc -l | tr -d ' ')
INTEGRATION_TESTS=$(find tests/ -name "*.rs" 2>/dev/null | wc -l | tr -d ' ')
BENCHMARK_TESTS=$(find src/ -name "*.rs" -exec grep -l "#\[bench\]" {} \; | wc -l | tr -d ' ')

echo "  - 单元测试模块: $UNIT_TESTS"
echo "  - 集成测试文件: $INTEGRATION_TESTS"
echo "  - 性能基准测试: $BENCHMARK_TESTS"

echo ""
echo "📋 第5步: 生成改进建议"
echo "--------------------"

echo "🎯 建议补充的测试类型:"
echo ""
echo "1. **API端点测试** (优先级: 🔥高)"
echo "   - HTTP请求/响应测试"
echo "   - 认证和授权测试"
echo "   - 输入验证和错误处理"
echo "   - Matrix协议合规性测试"
echo ""

echo "2. **服务层测试** (优先级: 🔥高)"
echo "   - 业务逻辑单元测试"
echo "   - 数据转换和处理测试"
echo "   - 异步操作和并发测试"
echo "   - 错误传播和恢复测试"
echo ""

echo "3. **集成测试** (优先级: 🔧中)"
echo "   - 端到端功能测试"
echo "   - 数据库集成测试"
echo "   - 外部服务集成测试"
echo "   - 性能和负载测试"
echo ""

echo "4. **创新测试** (优先级: ⚡新增)"
echo "   - 模糊测试 (Fuzz Testing)"
echo "   - 属性测试 (Property Testing)"
echo "   - 混沌工程测试"
echo "   - 安全渗透测试"
echo ""

# 清理临时文件
rm -f "$TEMP_FILE"

echo "📊 最终统计"
echo "=========="
echo -e "${BLUE}项目规模${NC}: $TOTAL_FILES 个Rust文件"
echo -e "${GREEN}测试覆盖${NC}: $FILES_WITH_TESTS 个文件有测试 ($COVERAGE_RATE%)"
echo -e "${YELLOW}待补充${NC}: $FILES_WITHOUT_TESTS 个文件需要测试"
echo ""
echo -e "${RED}优先修复模块${NC}:"
echo "  - API模块: $API_FILES_WITHOUT_TESTS 个文件"
echo "  - Service模块: $SERVICE_FILES_WITHOUT_TESTS 个文件"
echo "  - CLI模块: $CLI_FILES_WITHOUT_TESTS 个文件"
echo ""

if [ $COVERAGE_RATE -lt 70 ]; then
    echo -e "${RED}⚠️  测试覆盖率偏低，建议优先补充核心模块测试${NC}"
elif [ $COVERAGE_RATE -lt 85 ]; then
    echo -e "${YELLOW}📈 测试覆盖率良好，可继续完善边缘模块测试${NC}"
else
    echo -e "${GREEN}🎉 测试覆盖率优秀，可专注于测试质量提升${NC}"
fi

echo ""
echo "🚀 下一步行动计划"
echo "================"
echo "1. 运行 ./add_missing_tests.sh 自动为核心模块添加测试"
echo "2. 运行 ./create_innovative_tests.sh 创建创新测试"
echo "3. 运行 cargo test 验证所有测试通过"
echo "4. 运行 cargo test --doc 测试文档示例" 
