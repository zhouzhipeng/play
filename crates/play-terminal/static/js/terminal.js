/**
 * Web终端 JavaScript 客户端
 */

// DOM元素
const terminalElement = document.getElementById('terminal');
const outputElement = document.getElementById('terminal-output');
const inputElement = document.getElementById('terminal-input');
const promptElement = document.getElementById('prompt');
const clearButton = document.getElementById('clear-btn');
const settingsButton = document.getElementById('settings-btn');
const settingsModal = document.getElementById('settings-modal');
const closeSettingsButton = document.querySelector('.close');
const saveSettingsButton = document.getElementById('save-settings');
const cancelSettingsButton = document.getElementById('cancel-settings');
const fontSizeInput = document.getElementById('font-size');
const fontSizeValue = document.getElementById('font-size-value');
const themeSelect = document.getElementById('theme-select');
const cursorStyleSelect = document.getElementById('cursor-style');

// WebSocket
let socket;
let isConnected = false;
let commandHistory = [];
let historyIndex = -1;
let reconnectAttempts = 0;
const maxReconnectAttempts = 5;
const reconnectDelay = 2000;

// 终端配置
let terminalConfig = {
    fontSize: 14,
    theme: 'dark',
    cursorStyle: 'block'
};

// 初始化
function init() {
    // 加载保存的配置
    loadSettings();

    // 连接WebSocket
    connectWebSocket();

    // 设置事件监听器
    setupEventListeners();

    // 显示欢迎消息
    displayWelcomeMessage();

    // 焦点到输入框
    inputElement.focus();
}

// 连接WebSocket
function connectWebSocket() {
    // 获取当前主机和协议
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    const wsUrl = `${protocol}//${host}/ws`;

    try {
        socket = new WebSocket(wsUrl);

        // 连接打开
        socket.onopen = function() {
            isConnected = true;
            reconnectAttempts = 0;
            addSystemOutput('WebSocket连接已建立');

            // 请求终端信息
            requestTerminalInfo();
        };

        // 收到消息
        socket.onmessage = function(event) {
            handleWebSocketMessage(event.data);
        };

        // 连接关闭
        socket.onclose = function() {
            isConnected = false;
            addSystemOutput('WebSocket连接已关闭');

            // 尝试重连
            if (reconnectAttempts < maxReconnectAttempts) {
                reconnectAttempts++;
                addSystemOutput(`尝试重新连接 (${reconnectAttempts}/${maxReconnectAttempts})...`);
                setTimeout(connectWebSocket, reconnectDelay);
            } else {
                addSystemOutput('无法连接到服务器，请刷新页面重试');
            }
        };

        // 连接错误
        socket.onerror = function(error) {
            addErrorOutput('WebSocket错误: ' + error.message);
        };
    } catch (error) {
        addErrorOutput('无法创建WebSocket连接: ' + error.message);
    }
}

// 处理WebSocket消息
function handleWebSocketMessage(data) {
    try {
        const message = JSON.parse(data);

        // 根据消息类型处理
        switch (message.type) {
            case 'Output':
                // 命令输出
                const output = message.data;

                // 根据输出类型添加到终端
                switch (output.output_type) {
                    case 'Stdout':
                        addOutput(output.content);
                        break;
                    case 'Stderr':
                        addErrorOutput(output.content);
                        break;
                    case 'System':
                        addSystemOutput(output.content);
                        break;
                }

                // 如果命令完成，重新启用输入
                if (output.is_complete) {
                    enableInput();
                }
                break;

            case 'Error':
                // 错误消息
                addErrorOutput(message.data.message);
                enableInput();
                break;

            case 'Info':
                // 更新终端信息
                updateTerminalInfo(message.data);
                break;

            default:
                console.warn('未知的消息类型:', message.type);
        }
    } catch (error) {
        console.error('解析WebSocket消息时出错:', error);
        addErrorOutput('解析服务器消息时出错');
    }
}

// 设置事件监听器
function setupEventListeners() {
    // 命令输入
    inputElement.addEventListener('keydown', function(event) {
        if (event.key === 'Enter') {
            event.preventDefault();
            const command = inputElement.value.trim();

            if (command) {
                executeCommand(command);
            }
        } else if (event.key === 'ArrowUp') {
            event.preventDefault();
            navigateHistory(-1);
        } else if (event.key === 'ArrowDown') {
            event.preventDefault();
            navigateHistory(1);
        } else if (event.key === 'Tab') {
            event.preventDefault();
            // TODO: 实现命令补全
        }
    });

    // 确保终端始终保持焦点
    terminalElement.addEventListener('click', function() {
        inputElement.focus();
    });

    // 清除按钮
    clearButton.addEventListener('click', function() {
        clearTerminal();
    });

    // 设置按钮
    settingsButton.addEventListener('click', function() {
        openSettingsModal();
    });

    // 关闭设置模态框
    closeSettingsButton.addEventListener('click', function() {
        closeSettingsModal();
    });

    // 保存设置
    saveSettingsButton.addEventListener('click', function() {
        saveSettings();
        closeSettingsModal();
    });

    // 取消设置
    cancelSettingsButton.addEventListener('click', function() {
        closeSettingsModal();
    });

    // 字体大小滑块
    fontSizeInput.addEventListener('input', function() {
        const size = fontSizeInput.value;
        fontSizeValue.textContent = size + 'px';
    });

    // 点击模态框外部关闭
    window.addEventListener('click', function(event) {
        if (event.target === settingsModal) {
            closeSettingsModal();
        }
    });
}

// 执行命令
function executeCommand(commandText) {
    // 显示命令行
    addCommandLine(commandText);

    // 添加到历史记录
    commandHistory.unshift(commandText);
    historyIndex = -1;

    // 如果是内部命令，则在客户端处理
    if (handleInternalCommand(commandText)) {
        enableInput();
        return;
    }

    // 清空输入框
    inputElement.value = '';

    // 禁用输入，直到命令完成
    disableInput();

    // 解析命令和参数
    const parts = commandText.match(/(".*?"|'.*?'|[^"\s]+)+(?=\s*|\s*$)/g) || [];
    const command = parts[0];
    const args = parts.slice(1).map(arg => {
        // 去除引号
        if ((arg.startsWith('"') && arg.endsWith('"')) ||
            (arg.startsWith("'") && arg.endsWith("'"))) {
            return arg.slice(1, -1);
        }
        return arg;
    });

    // 如果WebSocket已连接，发送命令
    if (isConnected) {
        try {
            socket.send(JSON.stringify({
                type: 'Command',
                data: {
                    command: command,
                    args: args,
                    env: null,
                    cwd: null
                }
            }));
        } catch (error) {
            addErrorOutput('发送命令时出错: ' + error.message);
            enableInput();
        }
    } else {
        addErrorOutput('未连接到服务器');
        enableInput();
    }
}

// 处理内部命令
function handleInternalCommand(command) {
    const cmdLower = command.toLowerCase();

    // 清屏命令
    if (cmdLower === 'clear' || cmdLower === 'cls') {
        clearTerminal();
        return true;
    }

    // 帮助命令
    if (cmdLower === 'help') {
        showHelp();
        return true;
    }

    // 设置命令
    if (cmdLower === 'settings') {
        openSettingsModal();
        return true;
    }

    return false;
}

// 请求终端信息
function requestTerminalInfo() {
    if (isConnected) {
        try {
            socket.send(JSON.stringify({
                type: 'RequestInfo'
            }));
        } catch (error) {
            addErrorOutput('请求终端信息时出错: ' + error.message);
        }
    }
}

// 更新终端信息
function updateTerminalInfo(info) {
    // 更新提示符
    if (info.user && info.hostname) {
        promptElement.textContent = `${info.user}@${info.hostname}:${info.working_directory}$`;
    }
}

// 添加命令行到输出
function addCommandLine(command) {
    const element = document.createElement('div');
    element.className = 'output-line command-line';
    element.textContent = `${promptElement.textContent} ${command}`;
    outputElement.appendChild(element);
    scrollToBottom();
}

// 添加普通输出
function addOutput(text) {
    const element = document.createElement('div');
    element.className = 'output-line';
    element.textContent = text;
    outputElement.appendChild(element);
    scrollToBottom();
}

// 添加错误输出
function addErrorOutput(text) {
    const element = document.createElement('div');
    element.className = 'output-line error-output';
    element.textContent = text;
    outputElement.appendChild(element);
    scrollToBottom();
}

// 添加系统输出
function addSystemOutput(text) {
    const element = document.createElement('div');
    element.className = 'output-line system-output';
    element.textContent = text;
    outputElement.appendChild(element);
    scrollToBottom();
}

// 清除终端
function clearTerminal() {
    outputElement.innerHTML = '';
    inputElement.value = '';
    inputElement.focus();
}

// 显示欢迎消息
function displayWelcomeMessage() {
    addSystemOutput('欢迎使用Web终端！');
    addSystemOutput('输入 "help" 获取帮助。');
}

// 显示帮助信息
function showHelp() {
    addSystemOutput('=== Web终端帮助 ===');
    addSystemOutput('可用的内部命令:');
    addSystemOutput('  help     - 显示此帮助');
    addSystemOutput('  clear    - 清除终端');
    addSystemOutput('  settings - 打开设置');
    addSystemOutput('其他任何命令都将发送到服务器执行。');
}

// 导航命令历史记录
function navigateHistory(direction) {
    if (commandHistory.length === 0) {
        return;
    }

    historyIndex += direction;

    if (historyIndex < -1) {
        historyIndex = commandHistory.length - 1;
    } else if (historyIndex >= commandHistory.length) {
        historyIndex = -1;
    }

    if (historyIndex === -1) {
        inputElement.value = '';
    } else {
        inputElement.value = commandHistory[historyIndex];
    }

    // 将光标移到末尾
    setTimeout(() => {
        inputElement.selectionStart = inputElement.selectionEnd = inputElement.value.length;
    }, 0);
}

// 禁用输入
function disableInput() {
    inputElement.disabled = true;
}

// 启用输入
function enableInput() {
    inputElement.disabled = false;
    inputElement.focus();
}

// 滚动到底部
function scrollToBottom() {
    terminalElement.scrollTop = terminalElement.scrollHeight;
}

// 打开设置模态框
function openSettingsModal() {
    // 更新设置UI
    fontSizeInput.value = terminalConfig.fontSize;
    fontSizeValue.textContent = terminalConfig.fontSize + 'px';
    themeSelect.value = terminalConfig.theme;
    cursorStyleSelect.value = terminalConfig.cursorStyle;

    // 显示模态框
    settingsModal.style.display = 'flex';
}

// 关闭设置模态框
function closeSettingsModal() {
    settingsModal.style.display = 'none';
    inputElement.focus();
}

// 保存设置
function saveSettings() {
    terminalConfig.fontSize = parseInt(fontSizeInput.value);
    terminalConfig.theme = themeSelect.value;
    terminalConfig.cursorStyle = cursorStyleSelect.value;

    // 应用设置
    applySettings();

    // 保存到本地存储
    localStorage.setItem('terminal-config', JSON.stringify(terminalConfig));
}

// 加载设置
function loadSettings() {
    const savedConfig = localStorage.getItem('terminal-config');

    if (savedConfig) {
        try {
            const config = JSON.parse(savedConfig);
            terminalConfig = { ...terminalConfig, ...config };
        } catch (error) {
            console.error('加载设置时出错:', error);
        }
    }

    // 应用设置
    applySettings();
}

// 应用设置
function applySettings() {
    // 设置字体大小
    terminalElement.style.fontSize = terminalConfig.fontSize + 'px';

    // 设置主题
    document.body.classList.remove('theme-dark', 'theme-light', 'theme-matrix');
    document.body.classList.add('theme-' + terminalConfig.theme);

    // 设置光标样式
    inputElement.classList.remove('cursor-block', 'cursor-underline', 'cursor-bar');
    inputElement.classList.add('cursor-' + terminalConfig.cursorStyle);
    inputElement.classList.add('cursor-blink');
}

// 初始化终端
window.addEventListener('DOMContentLoaded', init);