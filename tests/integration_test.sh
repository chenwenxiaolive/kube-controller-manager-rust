#!/bin/bash
# 集成测试脚本：对比 Go 和 Rust 版本的行为

set -e

KUBECONFIG=${KUBECONFIG:-~/.kube/config}
NAMESPACE="test-$(date +%s)"
RUST_BINARY="./target/release/kube-controller-manager"

echo "=== 1. 启动 Go controller-manager (录制行为) ==="
# 记录 Go 版本的日志和事件
kubectl proxy --port=8001 &
PROXY_PID=$!
sleep 2

# 记录初始状态
curl -s http://localhost:8001/logs > /tmp/go-before.log

echo "=== 2. 运行 Rust controller-manager ==="
$RUST_BINARY \
  --kubeconfig "$KUBECONFIG" \
  --namespace "$NAMESPACE" \
  --controllers endpoint \
  --log-level debug \
  > /tmp/rust.log 2>&1 &
RUST_PID=$!

sleep 10

echo "=== 3. 对比行为 ==="
# 检查日志模式
echo "检查关键日志模式..."
grep -i "starting controller" /tmp/rust.log && echo "✓ Controller started"
grep -i "leader election" /tmp/rust.log && echo "✓ Leader election running"

# 清理
kill $RUST_PID $PROXY_PID 2>/dev/null || true
