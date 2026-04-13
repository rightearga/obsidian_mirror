// 分享链接管理
const ShareModal = {
    currentNotePath: null,
    
    // 打开分享模态框
    open(notePath) {
        this.currentNotePath = notePath;
        document.getElementById('share-modal').style.display = 'block';
        document.body.style.overflow = 'hidden';
    },
    
    // 关闭分享模态框
    close() {
        document.getElementById('share-modal').style.display = 'none';
        document.body.style.overflow = '';
        this.reset();
    },
    
    // 切换密码输入框
    togglePassword() {
        const enabled = document.getElementById('share-password-enabled').checked;
        const passwordInput = document.getElementById('share-password');
        passwordInput.style.display = enabled ? 'block' : 'none';
        if (!enabled) {
            passwordInput.value = '';
        }
    },
    
    // 切换访问次数限制输入框
    toggleLimit() {
        const enabled = document.getElementById('share-limit-enabled').checked;
        const limitInput = document.getElementById('share-limit');
        limitInput.style.display = enabled ? 'block' : 'none';
        if (!enabled) {
            limitInput.value = '';
        }
    },
    
    // 创建分享链接
    async createShare(event) {
        event.preventDefault();
        
        const form = document.getElementById('share-form');
        const submitBtn = form.querySelector('button[type="submit"]');
        
        // 禁用提交按钮
        submitBtn.disabled = true;
        submitBtn.textContent = '生成中...';
        
        try {
            // 构建请求数据
            const data = {
                note_path: this.currentNotePath
            };
            
            // 过期时间
            const expires = document.getElementById('share-expires').value;
            if (expires) {
                data.expires_in_seconds = parseInt(expires);
            }
            
            // 密码
            if (document.getElementById('share-password-enabled').checked) {
                const password = document.getElementById('share-password').value.trim();
                if (password) {
                    data.password = password;
                }
            }
            
            // 访问次数限制
            if (document.getElementById('share-limit-enabled').checked) {
                const limit = document.getElementById('share-limit').value;
                if (limit) {
                    data.max_visits = parseInt(limit);
                }
            }
            
            // 发送请求
            const response = await fetch('/api/share/create', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                credentials: 'include',
                body: JSON.stringify(data)
            });
            
            if (!response.ok) {
                const error = await response.json();
                throw new Error(error.error || '创建分享链接失败');
            }
            
            const result = await response.json();
            
            // 显示结果
            this.showResult(result);
            
        } catch (error) {
            alert('创建分享链接失败: ' + error.message);
            console.error('分享创建失败:', error);
        } finally {
            // 恢复提交按钮
            submitBtn.disabled = false;
            submitBtn.textContent = '生成分享链接';
        }
        
        return false;
    },
    
    // 显示分享结果
    showResult(result) {
        // 隐藏表单，显示结果
        document.getElementById('share-form').style.display = 'none';
        document.getElementById('share-result').style.display = 'block';
        
        // 设置分享链接
        document.getElementById('share-url').value = result.share_url;
        
        // 设置过期信息
        const expiresInfo = document.getElementById('share-expires-info');
        if (result.expires_at) {
            const expiresDate = new Date(result.expires_at);
            expiresInfo.textContent = `过期时间: ${expiresDate.toLocaleString('zh-CN')}`;
        } else {
            expiresInfo.textContent = '该链接永久有效';
        }
    },
    
    // 复制链接
    async copyUrl() {
        const urlInput = document.getElementById('share-url');
        const copyBtn = event.target;
        
        try {
            await navigator.clipboard.writeText(urlInput.value);
            
            // 显示复制成功提示
            const originalText = copyBtn.textContent;
            copyBtn.textContent = '✓ 已复制';
            copyBtn.classList.add('copied');
            
            setTimeout(() => {
                copyBtn.textContent = originalText;
                copyBtn.classList.remove('copied');
            }, 2000);
        } catch (error) {
            // 降级方案：选中文本
            urlInput.select();
            urlInput.setSelectionRange(0, 99999);
            
            try {
                document.execCommand('copy');
                alert('链接已复制到剪贴板');
            } catch (e) {
                alert('复制失败，请手动复制链接');
            }
        }
    },
    
    // 在新标签页打开链接
    openUrl() {
        const url = document.getElementById('share-url').value;
        window.open(url, '_blank');
    },
    
    // 重置表单
    reset() {
        document.getElementById('share-form').style.display = 'block';
        document.getElementById('share-result').style.display = 'none';
        document.getElementById('share-form').reset();
        document.getElementById('share-password').style.display = 'none';
        document.getElementById('share-limit').style.display = 'none';
        document.getElementById('share-password-enabled').checked = false;
        document.getElementById('share-limit-enabled').checked = false;
    }
};

// ESC 键关闭模态框
document.addEventListener('keydown', function(event) {
    if (event.key === 'Escape') {
        const shareModal = document.getElementById('share-modal');
        if (shareModal && shareModal.style.display === 'block') {
            ShareModal.close();
        }
    }
});
