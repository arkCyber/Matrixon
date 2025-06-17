# Matrixon 房间创建与通信 curl 命令指南

> **作者**: arkSong <arksong2018@gmail.com>  
> **版本**: 0.11.0-alpha  
> **日期**: 2024-01-15  
> **性能目标**: <50ms 响应时间，支持 200k+ 并发连接

## 📋 目录

1. [服务器状态检查](#1-服务器状态检查)
2. [用户注册与登录](#2-用户注册与登录)
3. [房间创建](#3-房间创建)
4. [房间加入](#4-房间加入)
5. [消息发送](#5-消息发送)
6. [消息获取](#6-消息获取)
7. [同步操作](#7-同步操作)
8. [性能测试](#8-性能测试)
9. [完整示例流程](#9-完整示例流程)

---

## 1. 服务器状态检查

### 健康检查
```bash
# 基本健康检查
curl -s http://localhost:6167/health | jq .

# 带响应时间测量
curl -s -w "Response time: %{time_total}s\n" http://localhost:6167/health | jq .
```

**预期响应**:
```json
{
  "status": "healthy",
  "service": "matrixon",
  "version": "0.11.0-alpha",
  "timestamp": "2024-01-15T10:30:00Z",
  "uptime_ms": 1234
}
```

### 指标监控
```bash
# 获取 Prometheus 格式指标
curl -s http://localhost:6167/metrics

# 检查特定指标
curl -s http://localhost:6167/metrics | grep matrixon_requests_total
```

### Matrix API 版本检查
```bash
# 获取支持的 Matrix 客户端 API 版本
curl -s http://localhost:6167/_matrix/client/versions | jq .

# 获取服务器能力
curl -s http://localhost:6167/_matrix/client/r0/capabilities | jq .
```

---

## 2. 用户注册与登录

### 用户注册
```bash
# 注册新用户 Alice
curl -X POST http://localhost:6167/_matrix/client/r0/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "password": "secure_password_123",
    "device_id": "ALICE_DEVICE",
    "initial_device_display_name": "Alice Device"
  }' | jq .

# 注册用户 Bob
curl -X POST http://localhost:6167/_matrix/client/r0/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "bob",
    "password": "secure_password_456",
    "device_id": "BOB_DEVICE",
    "initial_device_display_name": "Bob Device"
  }' | jq .
```

**预期响应**:
```json
{
  "user_id": "@alice:localhost",
  "access_token": "matx_access_token_alice_1705312200",
  "device_id": "ALICE_DEVICE",
  "home_server": "localhost"
}
```

### 用户登录
```bash
# 用户登录
curl -X POST http://localhost:6167/_matrix/client/r0/login \
  -H "Content-Type: application/json" \
  -d '{
    "type": "m.login.password",
    "user": "alice",
    "password": "secure_password_123",
    "device_id": "ALICE_LOGIN_DEVICE"
  }' | jq .
```

### 检查用户身份
```bash
# 使用访问令牌检查身份 (替换 YOUR_ACCESS_TOKEN)
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  http://localhost:6167/_matrix/client/r0/account/whoami | jq .
```

---

## 3. 房间创建

### 基本房间创建
```bash
# 创建简单房间
curl -X POST http://localhost:6167/_matrix/client/r0/createRoom \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My Test Room",
    "topic": "A room for testing Matrixon features"
  }' | jq .
```

### 高级房间创建
```bash
# 创建带别名的公开房间
curl -X POST http://localhost:6167/_matrix/client/r0/createRoom \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Matrixon Development Room",
    "topic": "Discussing Matrixon development and features",
    "preset": "public_chat",
    "visibility": "public",
    "room_alias_name": "matrixon-dev",
    "room_version": "9",
    "invite": ["@bob:localhost"],
    "power_level_content_override": {
      "users": {
        "@alice:localhost": 100
      }
    }
  }' | jq .
```

**预期响应**:
```json
{
  "room_id": "!abcdef123456:localhost",
  "room_alias": "#matrixon-dev:localhost"
}
```

### 不同类型房间创建
```bash
# 私人聊天房间
curl -X POST http://localhost:6167/_matrix/client/r0/createRoom \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "preset": "private_chat",
    "invite": ["@bob:localhost"]
  }' | jq .

# 受信任的私人房间
curl -X POST http://localhost:6167/_matrix/client/r0/createRoom \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "preset": "trusted_private_chat",
    "invite": ["@bob:localhost"],
    "is_direct": true
  }' | jq .
```

---

## 4. 房间加入

### 通过房间 ID 加入
```bash
# 加入房间 (替换 ROOM_ID)
curl -X POST http://localhost:6167/_matrix/client/r0/join/!ROOM_ID:localhost \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{}' | jq .
```

### 通过房间别名加入
```bash
# 通过别名加入房间
curl -X POST "http://localhost:6167/_matrix/client/r0/join/%23matrixon-dev%3Alocalhost" \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{}' | jq .
```

**预期响应**:
```json
{
  "room_id": "!abcdef123456:localhost"
}
```

### 获取已加入房间列表
```bash
# 获取用户已加入的房间
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  http://localhost:6167/_matrix/client/r0/joined_rooms | jq .
```

---

## 5. 消息发送

### 发送文本消息
```bash
# 发送基本文本消息 (替换 ROOM_ID 和事务 ID)
TXN_ID="txn_$(date +%s)_1"
curl -X PUT "http://localhost:6167/_matrix/client/r0/rooms/!ROOM_ID:localhost/send/m.room.message/$TXN_ID" \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "msgtype": "m.text",
    "body": "Hello, Matrixon! 👋"
  }' | jq .
```

### 发送格式化消息
```bash
# 发送 HTML 格式化消息
TXN_ID="txn_$(date +%s)_2"
curl -X PUT "http://localhost:6167/_matrix/client/r0/rooms/!ROOM_ID:localhost/send/m.room.message/$TXN_ID" \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "msgtype": "m.text",
    "body": "Hello **world**! This is a test message.",
    "format": "org.matrix.custom.html",
    "formatted_body": "Hello <strong>world</strong>! This is a test message."
  }' | jq .
```

### 发送代码消息
```bash
# 发送代码片段
TXN_ID="txn_$(date +%s)_3"
curl -X PUT "http://localhost:6167/_matrix/client/r0/rooms/!ROOM_ID:localhost/send/m.room.message/$TXN_ID" \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "msgtype": "m.text",
    "body": "fn main() {\n    println!(\"Hello, Matrixon!\");\n}",
    "format": "org.matrix.custom.html",
    "formatted_body": "<pre><code class=\"language-rust\">fn main() {\n    println!(\"Hello, Matrixon!\");\n}</code></pre>"
  }' | jq .
```

### 发送通知消息
```bash
# 发送通知类型消息
TXN_ID="txn_$(date +%s)_4"
curl -X PUT "http://localhost:6167/_matrix/client/r0/rooms/!ROOM_ID:localhost/send/m.room.message/$TXN_ID" \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "msgtype": "m.notice",
    "body": "🤖 System: Room statistics updated successfully!"
  }' | jq .
```

**预期响应**:
```json
{
  "event_id": "$abc123def456:localhost"
}
```

---

## 6. 消息获取

### 获取房间消息历史
```bash
# 获取最近 10 条消息
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  "http://localhost:6167/_matrix/client/r0/rooms/!ROOM_ID:localhost/messages?limit=10" | jq .

# 获取特定数量的消息
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  "http://localhost:6167/_matrix/client/r0/rooms/!ROOM_ID:localhost/messages?limit=20&dir=b" | jq .
```

**预期响应**:
```json
{
  "chunk": [
    {
      "event_id": "$abc123:localhost",
      "sender": "@alice:localhost",
      "type": "m.room.message",
      "content": {
        "msgtype": "m.text",
        "body": "Hello, Matrixon! 👋"
      },
      "origin_server_ts": 1705312200000,
      "room_id": "!room123:localhost"
    }
  ],
  "start": "t1-start",
  "end": "t1-end"
}
```

---

## 7. 同步操作

### 基本同步
```bash
# 初始同步 (获取所有数据)
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  "http://localhost:6167/_matrix/client/r0/sync?timeout=0" | jq .

# 增量同步 (使用 since 参数)
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  "http://localhost:6167/_matrix/client/r0/sync?since=sync_token_1705312200&timeout=5000" | jq .
```

### 长轮询同步
```bash
# 30 秒长轮询
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  "http://localhost:6167/_matrix/client/r0/sync?timeout=30000" | jq .
```

**预期响应**:
```json
{
  "next_batch": "sync_token_1705312300",
  "rooms": {
    "join": {
      "!room123:localhost": {
        "timeline": {
          "events": [
            {
              "event_id": "$abc123:localhost",
              "sender": "@alice:localhost",
              "type": "m.room.message",
              "content": {
                "msgtype": "m.text",
                "body": "Hello!"
              },
              "origin_server_ts": 1705312200000,
              "room_id": "!room123:localhost"
            }
          ],
          "limited": false,
          "prev_batch": "prev_batch_token"
        },
        "state": {
          "events": []
        }
      }
    }
  }
}
```

---

## 8. 性能测试

### 批量消息发送性能测试
```bash
# 发送 10 条消息并测量响应时间
for i in {1..10}; do
  start_time=$(date +%s%N)
  TXN_ID="perf_test_$(date +%s)_$i"
  
  curl -s -X PUT "http://localhost:6167/_matrix/client/r0/rooms/!ROOM_ID:localhost/send/m.room.message/$TXN_ID" \
    -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
      \"msgtype\": \"m.text\",
      \"body\": \"Performance test message #$i\"
    }" > /dev/null
  
  end_time=$(date +%s%N)
  duration=$(( (end_time - start_time) / 1000000 ))
  echo "Message $i sent in ${duration}ms"
done
```

### 并发测试
```bash
# 使用 GNU parallel 进行并发测试 (需要安装 parallel)
seq 1 50 | parallel -j 10 '
  TXN_ID="concurrent_test_$(date +%s)_{}"
  start_time=$(date +%s%N)
  
  curl -s -X PUT "http://localhost:6167/_matrix/client/r0/rooms/!ROOM_ID:localhost/send/m.room.message/$TXN_ID" \
    -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
      \"msgtype\": \"m.text\",
      \"body\": \"Concurrent test message #{}\",
      \"timestamp\": $(date +%s)
    }" > /dev/null
  
  end_time=$(date +%s%N)
  duration=$(( (end_time - start_time) / 1000000 ))
  echo "Concurrent message {} sent in ${duration}ms"
'
```

---

## 9. 完整示例流程

### 完整的聊天室创建和通信流程
```bash
#!/bin/bash

# 设置服务器地址
MATRIXON_URL="http://localhost:6167"

echo "🚀 开始完整的 Matrixon 房间通信测试"

# 1. 检查服务器状态
echo "📡 检查服务器状态..."
curl -s "$MATRIXON_URL/health" | jq .

# 2. 注册用户 Alice
echo "👤 注册用户 Alice..."
ALICE_RESPONSE=$(curl -s -X POST "$MATRIXON_URL/_matrix/client/r0/register" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice_demo",
    "password": "alice_password_123",
    "device_id": "ALICE_DEMO_DEVICE"
  }')

ALICE_TOKEN=$(echo "$ALICE_RESPONSE" | jq -r '.access_token')
echo "Alice 注册成功，Token: $ALICE_TOKEN"

# 3. 注册用户 Bob
echo "👤 注册用户 Bob..."
BOB_RESPONSE=$(curl -s -X POST "$MATRIXON_URL/_matrix/client/r0/register" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "bob_demo",
    "password": "bob_password_456",
    "device_id": "BOB_DEMO_DEVICE"
  }')

BOB_TOKEN=$(echo "$BOB_RESPONSE" | jq -r '.access_token')
echo "Bob 注册成功，Token: $BOB_TOKEN"

# 4. Alice 创建房间
echo "🏠 Alice 创建聊天室..."
ROOM_RESPONSE=$(curl -s -X POST "$MATRIXON_URL/_matrix/client/r0/createRoom" \
  -H "Authorization: Bearer $ALICE_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Demo Chat Room",
    "topic": "Demonstration of Matrixon messaging",
    "preset": "public_chat",
    "room_alias_name": "demo-chat"
  }')

ROOM_ID=$(echo "$ROOM_RESPONSE" | jq -r '.room_id')
echo "房间创建成功，Room ID: $ROOM_ID"

# 5. Bob 加入房间
echo "🚪 Bob 加入房间..."
curl -s -X POST "$MATRIXON_URL/_matrix/client/r0/join/$ROOM_ID" \
  -H "Authorization: Bearer $BOB_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{}' | jq .

# 6. Alice 发送欢迎消息
echo "💬 Alice 发送欢迎消息..."
TXN_ID1="demo_$(date +%s)_1"
curl -s -X PUT "$MATRIXON_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/$TXN_ID1" \
  -H "Authorization: Bearer $ALICE_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "msgtype": "m.text",
    "body": "🎉 欢迎来到 Matrixon 演示聊天室！"
  }' | jq .

# 7. Bob 回复消息
echo "💬 Bob 回复消息..."
TXN_ID2="demo_$(date +%s)_2"
curl -s -X PUT "$MATRIXON_URL/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/$TXN_ID2" \
  -H "Authorization: Bearer $BOB_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "msgtype": "m.text",
    "body": "👋 谢谢 Alice！Matrixon 运行得很棒！"
  }' | jq .

# 8. 获取聊天历史
echo "📖 获取聊天历史..."
curl -s "$MATRIXON_URL/_matrix/client/r0/rooms/$ROOM_ID/messages?limit=10" \
  -H "Authorization: Bearer $ALICE_TOKEN" | jq .

# 9. 同步检查
echo "🔄 执行同步操作..."
curl -s "$MATRIXON_URL/_matrix/client/r0/sync?timeout=0" \
  -H "Authorization: Bearer $ALICE_TOKEN" | jq '.rooms.join | keys[]'

echo "✅ 演示完成！"
echo "📝 房间 ID: $ROOM_ID"
echo "🔑 Alice Token: $ALICE_TOKEN"
echo "🔑 Bob Token: $BOB_TOKEN"
```

---

## 💡 提示和最佳实践

### 性能优化建议
1. **使用长轮询**: 设置适当的 `timeout` 参数进行同步
2. **批量操作**: 尽可能批量发送消息
3. **缓存令牌**: 保存访问令牌以避免重复登录
4. **错误处理**: 检查 HTTP 状态码和响应内容

### 常见错误排查
```bash
# 检查认证错误
curl -v -H "Authorization: Bearer INVALID_TOKEN" \
  http://localhost:6167/_matrix/client/r0/account/whoami

# 检查房间不存在错误
curl -v -X POST http://localhost:6167/_matrix/client/r0/join/!nonexistent:localhost \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{}'
```

### 监控和调试
```bash
# 实时监控日志
tail -f room_communication_test.log

# 检查响应时间
curl -w "总时间: %{time_total}s\n连接时间: %{time_connect}s\n" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  http://localhost:6167/_matrix/client/r0/sync?timeout=0

# 检查服务器指标
watch -n 1 'curl -s http://localhost:6167/metrics | grep matrixon_requests_total'
```

---

## 🔗 相关资源

- [Matrix Client-Server API 规范](https://matrix.org/docs/spec/client_server/latest/)
- [Matrixon 项目文档](https://github.com/arksong2018/Matrixon)
- [Matrix 协议介绍](https://matrix.org/docs/guides/)

---

**注意**: 替换示例中的 `YOUR_ACCESS_TOKEN`、`ROOM_ID` 等占位符为实际值。确保 Matrixon 服务器正在 `localhost:6167` 运行。 
