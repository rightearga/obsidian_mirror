# obsidian_mirror 构建脚本（v1.6.0）
# 包含服务端和 WASM 模块的构建命令

.PHONY: help wasm wasm-dev server build all clean test

# 默认目标：显示帮助
help:
	@echo "obsidian_mirror 构建命令："
	@echo ""
	@echo "  make wasm        构建 WASM 模块（release 模式，需安装 wasm-pack）"
	@echo "  make wasm-dev    构建 WASM 模块（debug 模式，更快，适合开发）"
	@echo "  make server      构建服务端（release 模式）"
	@echo "  make build       构建全部（WASM + 服务端）"
	@echo "  make test        运行所有测试（包含 WASM crate 单元测试）"
	@echo "  make clean       清理所有构建产物"
	@echo ""
	@echo "安装 wasm-pack："
	@echo "  cargo install wasm-pack"

# ─── WASM 构建 ──────────────────────────────────────────────────────────────

# 构建 WASM 模块（release 模式）
# 输出到 static/wasm/：.wasm 二进制 + wasm-pack 生成的 JS 胶水代码
wasm:
	@echo "🔨 构建 WASM 模块（release）..."
	wasm-pack build crates/wasm \
	    --target web \
	    --out-dir ../../static/wasm \
	    --out-name obsidian_mirror_wasm \
	    --release
	@echo "✅ WASM 模块已输出到 static/wasm/"
	@ls -lh static/wasm/obsidian_mirror_wasm_bg.wasm 2>/dev/null || true

# 构建 WASM 模块（debug 模式，速度更快，适合开发调试）
wasm-dev:
	@echo "🔨 构建 WASM 模块（debug）..."
	wasm-pack build crates/wasm \
	    --target web \
	    --out-dir ../../static/wasm \
	    --out-name obsidian_mirror_wasm
	@echo "✅ WASM 模块（debug）已输出到 static/wasm/"

# ─── 服务端构建 ─────────────────────────────────────────────────────────────

# 构建服务端（release 模式）
server:
	@echo "🔨 构建服务端（release）..."
	cargo build --release
	@echo "✅ 服务端已构建：target/release/obsidian_mirror"

# ─── 组合目标 ────────────────────────────────────────────────────────────────

# 构建全部：WASM + 服务端
build: wasm server
	@echo "🎉 全部构建完成"

# 运行所有测试（服务端 + WASM crate 单元测试）
test:
	@echo "🧪 运行所有测试..."
	cargo test --workspace
	@echo "✅ 全部测试通过"

# 清理构建产物
clean:
	@echo "🧹 清理构建产物..."
	cargo clean
	@rm -f static/wasm/obsidian_mirror_wasm_bg.wasm
	@rm -f static/wasm/obsidian_mirror_wasm.js
	@rm -f static/wasm/obsidian_mirror_wasm_bg.js
	@rm -f static/wasm/obsidian_mirror_wasm.d.ts
	@rm -f static/wasm/package.json
	@echo "✅ 清理完成"
