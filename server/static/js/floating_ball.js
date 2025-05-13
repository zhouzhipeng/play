/**
 * 浮动导航小球
 * 一个可以悬浮在页面角落的导航小球，点击后跳转到首页
 */
// 确保在DOM加载完成后执行
(function() {
    // 创建初始化函数
    function initFloatingBall() {
        // 创建小球元素
        const navBall = document.createElement('div');
        navBall.id = 'floating-nav-ball';

        // 创建小球内的图标
        const homeIcon = document.createElement('div');
        homeIcon.className = 'home-icon';

        // 将图标添加到小球中
        navBall.appendChild(homeIcon);

        // 添加样式
        const style = document.createElement('style');
        style.textContent = `
      #floating-nav-ball {
        position: fixed;
        bottom: 20px;
        right: 20px;
        width: 60px;
        height: 60px;
        background-color: #3498db;
        border-radius: 50%;
        box-shadow: 0 4px 8px rgba(0, 0, 0, 0.3);
        display: flex;
        justify-content: center;
        align-items: center;
        cursor: pointer;
        z-index: 9999;
        transition: all 0.3s ease;
      }
      
      #floating-nav-ball:hover {
        transform: scale(1.1);
        background-color: #2980b9;
      }
      
      #floating-nav-ball:active {
        transform: scale(0.95);
      }
      
      .home-icon {
        width: 24px;
        height: 24px;
        position: relative;
      }
      
      .home-icon:before {
        content: '';
        position: absolute;
        top: 0;
        left: 0;
        width: 0;
        height: 0;
        border-left: 12px solid transparent;
        border-right: 12px solid transparent;
        border-bottom: 12px solid white;
      }
      
      .home-icon:after {
        content: '';
        position: absolute;
        top: 12px;
        left: 3px;
        width: 18px;
        height: 12px;
        background-color: white;
        border-radius: 3px 3px 0 0;
      }
    `;

        // 添加点击事件，跳转到首页
        navBall.addEventListener('click', function() {
            window.location.href = '/';
        });

        // 将样式和小球添加到文档中
        if (document.head) document.head.appendChild(style);
        if (document.body) document.body.appendChild(navBall);

        // 添加拖拽功能
        let isDragging = false;
        let offsetX, offsetY;

        navBall.addEventListener('mousedown', function(e) {
            isDragging = true;
            offsetX = e.clientX - navBall.getBoundingClientRect().left;
            offsetY = e.clientY - navBall.getBoundingClientRect().top;

            // 防止拖动时触发点击事件
            navBall.style.transition = 'none';
        });

        document.addEventListener('mousemove', function(e) {
            if (isDragging) {
                const x = e.clientX - offsetX;
                const y = e.clientY - offsetY;

                // 确保小球不会超出屏幕边界
                const maxX = window.innerWidth - navBall.offsetWidth;
                const maxY = window.innerHeight - navBall.offsetHeight;

                navBall.style.left = Math.max(0, Math.min(x, maxX)) + 'px';
                navBall.style.right = 'auto';
                navBall.style.top = Math.max(0, Math.min(y, maxY)) + 'px';
                navBall.style.bottom = 'auto';
            }
        });

        document.addEventListener('mouseup', function() {
            if (isDragging) {
                isDragging = false;
                navBall.style.transition = 'all 0.3s ease';
            }
        });

        // 添加触摸支持，适配移动设备
        navBall.addEventListener('touchstart', function(e) {
            isDragging = true;
            offsetX = e.touches[0].clientX - navBall.getBoundingClientRect().left;
            offsetY = e.touches[0].clientY - navBall.getBoundingClientRect().top;
            navBall.style.transition = 'none';

            // 防止触摸时页面滚动
            e.preventDefault();
        }, { passive: false });

        document.addEventListener('touchmove', function(e) {
            if (isDragging) {
                const x = e.touches[0].clientX - offsetX;
                const y = e.touches[0].clientY - offsetY;

                const maxX = window.innerWidth - navBall.offsetWidth;
                const maxY = window.innerHeight - navBall.offsetHeight;

                navBall.style.left = Math.max(0, Math.min(x, maxX)) + 'px';
                navBall.style.right = 'auto';
                navBall.style.top = Math.max(0, Math.min(y, maxY)) + 'px';
                navBall.style.bottom = 'auto';

                // 防止触摸时页面滚动
                e.preventDefault();
            }
        }, { passive: false });

        document.addEventListener('touchend', function() {
            if (isDragging) {
                isDragging = false;
                navBall.style.transition = 'all 0.3s ease';
            }
        });
    }

    // 检查DOM是否已加载完成
    if (document.readyState === 'loading') {
        // 如果DOM还在加载中，监听DOMContentLoaded事件
        document.addEventListener('DOMContentLoaded', initFloatingBall);
    } else {
        // 如果DOM已经加载完成，直接执行
        initFloatingBall();
    }
})();