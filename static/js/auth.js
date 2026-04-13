/**
 * 认证相关的 JavaScript 功能
 */

// 获取当前用户信息并显示用户菜单
async function initializeAuth() {
    try {
        const response = await fetch('/api/auth/current-user');
        const data = await response.json();
        
        if (data.success && data.username) {
            // 显示用户菜单
            const userMenu = document.getElementById('user-menu');
            const usernameDisplay = document.getElementById('username-display');
            
            if (userMenu && usernameDisplay) {
                usernameDisplay.textContent = data.username;
                userMenu.style.display = 'block';
            }
        }
    } catch (error) {
        console.error('获取用户信息失败:', error);
    }
}

// 切换用户菜单下拉框
function toggleUserMenu() {
    const dropdown = document.getElementById('user-menu-dropdown');
    if (dropdown) {
        const isHidden = dropdown.style.display === 'none';
        dropdown.style.display = isHidden ? 'block' : 'none';
    }
}

// 登出功能
async function logout() {
    try {
        const response = await fetch('/api/auth/logout', {
            method: 'POST',
        });
        
        const data = await response.json();
        
        if (data.success) {
            // 登出成功，跳转到登录页
            window.location.href = '/login';
        } else {
            alert('登出失败: ' + (data.message || '未知错误'));
        }
    } catch (error) {
        console.error('登出错误:', error);
        alert('网络错误，请稍后重试');
    }
}

// 页面加载完成后初始化
document.addEventListener('DOMContentLoaded', function() {
    // 初始化认证
    initializeAuth();
    
    // 绑定用户菜单按钮点击事件
    const userMenuButton = document.getElementById('user-menu-button');
    if (userMenuButton) {
        userMenuButton.addEventListener('click', toggleUserMenu);
    }
    
    // 绑定登出按钮点击事件
    const logoutButton = document.getElementById('logout-button');
    if (logoutButton) {
        logoutButton.addEventListener('click', function(e) {
            e.preventDefault();
            if (confirm('确定要登出吗？')) {
                logout();
            }
        });
    }
    
    // 点击页面其他地方关闭用户菜单
    document.addEventListener('click', function(e) {
        const userMenu = document.getElementById('user-menu');
        const dropdown = document.getElementById('user-menu-dropdown');
        
        if (userMenu && dropdown && !userMenu.contains(e.target)) {
            dropdown.style.display = 'none';
        }
    });
});
