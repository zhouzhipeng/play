/**
 * 增强型浮动导航小球
 * 一个可以悬浮在页面角落的导航小球，点击后显示选项菜单，可以选择跳转到首页或重启服务
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

        // 创建选项菜单
        const menuContainer = document.createElement('div');
        menuContainer.id = 'nav-menu-container';
        menuContainer.style.display = 'none';

        // 创建选项
        const homeOption = document.createElement('div');
        homeOption.className = 'nav-menu-option';
        homeOption.textContent = '跳转到首页';

        const rebootOption = document.createElement('div');
        rebootOption.className = 'nav-menu-option';
        rebootOption.textContent = '重启服务';

        // 将选项添加到菜单中
        menuContainer.appendChild(rebootOption);
        menuContainer.appendChild(homeOption);


        // 创建Toast提示元素
        const toastElement = document.createElement('div');
        toastElement.id = 'toast-notification';
        toastElement.style.display = 'none';

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

      #nav-menu-container {
        position: fixed;
        background-color: white;
        border-radius: 8px;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
        padding: 10px 0;
        z-index: 9998;
        width: 150px;
      }
      
      .nav-menu-option {
        padding: 10px 16px;
        cursor: pointer;
        transition: background-color 0.2s ease;
      }
      
      .nav-menu-option:hover {
        background-color: #f5f5f5;
      }

      #toast-notification {
        position: fixed;
        bottom: 90px;
        right: 20px;
        padding: 12px 20px;
        background-color: #2ecc71;
        color: white;
        border-radius: 4px;
        box-shadow: 0 2px 10px rgba(0, 0, 0, 0.2);
        z-index: 10000;
        opacity: 0;
        transition: opacity 0.3s ease;
      }
      
      #toast-notification.show {
        opacity: 1;
      }
    `;

        // 添加点击事件，显示选项菜单
        navBall.addEventListener('click', function(e) {
            // 防止冒泡，以便能够检测到点击菜单外部区域
            e.stopPropagation();

            // 获取小球位置
            const ballRect = navBall.getBoundingClientRect();

            // 定位菜单位置（在小球上方）
            menuContainer.style.display = 'block';
            menuContainer.style.bottom = (window.innerHeight - ballRect.top + 10) + 'px';
            menuContainer.style.right = (window.innerWidth - ballRect.right + 10) + 'px';
        });

        // 点击首页选项，跳转到首页
        homeOption.addEventListener('click', function() {
            window.location.href = '/';
        });

        // 点击重启选项，调用API
        rebootOption.addEventListener('click', function() {
            // 隐藏菜单
            menuContainer.style.display = 'none';

            // 调用重启API
            fetch('/admin/reboot', {
                method: 'GET',
                headers: {
                    'Content-Type': 'application/json'
                }
            })
                .then(response => {
                    if (response.ok) {
                        // 显示成功提示
                        showToast('服务重启成功');
                    } else {
                        // 显示错误提示
                        showToast('服务重启失败', true);
                    }
                })
                .catch(error => {
                    console.error('重启服务出错:', error);
                    showToast('服务重启失败', true);
                });
        });

        // 点击页面其他区域关闭菜单
        document.addEventListener('click', function() {
            menuContainer.style.display = 'none';
        });

        // Toast提示函数
        function showToast(message, isError = false) {
            toastElement.textContent = message;
            toastElement.style.display = 'block';

            // 如果是错误提示，使用红色背景
            if (isError) {
                toastElement.style.backgroundColor = '#e74c3c';
            } else {
                toastElement.style.backgroundColor = '#2ecc71';
            }

            // 添加显示类名使其显示
            toastElement.classList.add('show');

            // 3秒后隐藏
            setTimeout(function() {
                toastElement.classList.remove('show');
                setTimeout(function() {
                    toastElement.style.display = 'none';
                }, 300);
            }, 3000);
        }

        // 将样式和元素添加到文档中
        if (document.head) document.head.appendChild(style);
        if (document.body) {
            document.body.appendChild(navBall);
            document.body.appendChild(menuContainer);
            document.body.appendChild(toastElement);
        }

        // 添加拖拽功能
        let isDragging = false;
        let offsetX, offsetY;
        let dragStartTime;
        const DRAG_THRESHOLD = 200; // 拖拽阈值，毫秒

        navBall.addEventListener('mousedown', function(e) {
            isDragging = false;
            offsetX = e.clientX - navBall.getBoundingClientRect().left;
            offsetY = e.clientY - navBall.getBoundingClientRect().top;
            dragStartTime = Date.now();

            // 防止拖动时触发点击事件
            navBall.style.transition = 'none';
        });

        document.addEventListener('mousemove', function(e) {
            if (dragStartTime && Date.now() - dragStartTime > DRAG_THRESHOLD) {
                isDragging = true;
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

        document.addEventListener('mouseup', function(e) {
            if (dragStartTime) {
                if (!isDragging && e.target === navBall || e.target === homeIcon) {
                    // 如果不是拖拽，而是点击，显示菜单
                    const ballRect = navBall.getBoundingClientRect();
                    menuContainer.style.display = 'block';
                    menuContainer.style.bottom = (window.innerHeight - ballRect.top + 10) + 'px';
                    menuContainer.style.right = (window.innerWidth - ballRect.right + 10) + 'px';
                }

                isDragging = false;
                dragStartTime = null;
                navBall.style.transition = 'all 0.3s ease';
            }
        });

        // 添加触摸支持，适配移动设备
        navBall.addEventListener('touchstart', function(e) {
            isDragging = false;
            offsetX = e.touches[0].clientX - navBall.getBoundingClientRect().left;
            offsetY = e.touches[0].clientY - navBall.getBoundingClientRect().top;
            dragStartTime = Date.now();
            navBall.style.transition = 'none';

            // 防止触摸时页面滚动
            e.preventDefault();
        }, { passive: false });

        document.addEventListener('touchmove', function(e) {
            if (dragStartTime && Date.now() - dragStartTime > DRAG_THRESHOLD) {
                isDragging = true;
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

        document.addEventListener('touchend', function(e) {
            if (dragStartTime) {
                if (!isDragging) {
                    // 如果不是拖拽，而是点击，显示菜单
                    const ballRect = navBall.getBoundingClientRect();
                    menuContainer.style.display = 'block';
                    menuContainer.style.bottom = (window.innerHeight - ballRect.top + 10) + 'px';
                    menuContainer.style.right = (window.innerWidth - ballRect.right + 10) + 'px';
                }

                isDragging = false;
                dragStartTime = null;
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