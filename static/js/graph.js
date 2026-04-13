// 关系图谱可视化逻辑

let graphNetwork = null;
let currentGraphDepth = 2;

/**
 * 打开图谱视图
 */
function toggleGraphView() {
    const modal = document.getElementById('graph-modal');
    if (modal.style.display === 'none' || modal.style.display === '') {
        openGraphView();
    } else {
        closeGraphView();
    }
}

/**
 * 打开图谱视图
 */
function openGraphView() {
    const modal = document.getElementById('graph-modal');
    modal.style.display = 'flex';
    
    // 加载图谱数据
    loadGraphData(currentGraphDepth);
    
    // 添加 ESC 键关闭
    document.addEventListener('keydown', handleGraphEsc);
}

/**
 * 关闭图谱视图
 */
function closeGraphView() {
    const modal = document.getElementById('graph-modal');
    modal.style.display = 'none';
    
    // 移除事件监听
    document.removeEventListener('keydown', handleGraphEsc);
    
    // 清理图谱
    if (graphNetwork) {
        graphNetwork.destroy();
        graphNetwork = null;
    }
}

/**
 * 处理 ESC 键
 */
function handleGraphEsc(e) {
    if (e.key === 'Escape') {
        closeGraphView();
    }
}

/**
 * 更新图谱深度
 */
function updateGraphDepth() {
    const select = document.getElementById('graph-depth');
    currentGraphDepth = parseInt(select.value);
    loadGraphData(currentGraphDepth);
}

/**
 * 加载图谱数据
 */
async function loadGraphData(depth) {
    const noteTitle = window.currentNoteTitle;
    console.log('[图谱] 当前笔记标题:', noteTitle);
    
    if (!noteTitle) {
        console.error('[图谱] 当前笔记标题未定义');
        const loading = document.getElementById('graph-loading');
        loading.innerHTML = `
            <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" style="color: var(--error-color);">
                <circle cx="12" cy="12" r="10"></circle>
                <line x1="12" y1="8" x2="12" y2="12"></line>
                <line x1="12" y1="16" x2="12.01" y2="16"></line>
            </svg>
            <p style="color: var(--error-color);">错误：无法获取当前笔记标题</p>
        `;
        return;
    }
    
    const container = document.getElementById('graph-container');
    const loading = document.getElementById('graph-loading');
    
    // 显示加载状态
    loading.style.display = 'flex';
    container.style.display = 'none';
    
    const apiUrl = `/api/graph?note=${encodeURIComponent(noteTitle)}&depth=${depth}`;
    console.log('[图谱] 请求 API:', apiUrl);
    
    try {
        const response = await fetch(apiUrl);
        console.log('[图谱] 响应状态:', response.status);
        
        if (!response.ok) {
            const errorText = await response.text();
            console.error('[图谱] API 错误响应:', errorText);
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        
        const data = await response.json();
        console.log('[图谱] 接收到数据:', data);
        console.log('[图谱] 节点数量:', data.nodes.length);
        console.log('[图谱] 边数量:', data.edges.length);
        
        // 隐藏加载状态
        loading.style.display = 'none';
        container.style.display = 'block';
        
        // 渲染图谱
        renderGraph(data, noteTitle);
        
    } catch (error) {
        console.error('[图谱] 加载图谱失败:', error);
        loading.innerHTML = `
            <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" style="color: var(--error-color);">
                <circle cx="12" cy="12" r="10"></circle>
                <line x1="12" y1="8" x2="12" y2="12"></line>
                <line x1="12" y1="16" x2="12.01" y2="16"></line>
            </svg>
            <p style="color: var(--error-color);">加载图谱失败: ${error.message}</p>
        `;
    }
}

/**
 * 渲染图谱
 */
function renderGraph(data, currentNoteTitle) {
    console.log('[图谱] 开始渲染图谱');
    const container = document.getElementById('graph-container');
    
    // 检查 vis.js 是否已加载
    if (typeof vis === 'undefined') {
        console.error('[图谱] vis.js 库未加载');
        const loading = document.getElementById('graph-loading');
        loading.innerHTML = `
            <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" style="color: var(--error-color);">
                <circle cx="12" cy="12" r="10"></circle>
                <line x1="12" y1="8" x2="12" y2="12"></line>
                <line x1="12" y1="16" x2="12.01" y2="16"></line>
            </svg>
            <p style="color: var(--error-color);">错误：可视化库未加载</p>
        `;
        loading.style.display = 'flex';
        container.style.display = 'none';
        return;
    }
    
    // 清理旧图谱
    if (graphNetwork) {
        graphNetwork.destroy();
    }
    
    console.log('[图谱] 准备节点数据...');
    // 准备节点数据
    const nodes = data.nodes.map(node => ({
        id: node.id,
        label: node.label,
        title: node.title,
        // 高亮当前笔记节点
        color: node.label === currentNoteTitle ? {
            background: '#4a9eff',
            border: '#2563eb',
            highlight: {
                background: '#3b82f6',
                border: '#1d4ed8'
            }
        } : {
            background: '#6b7280',
            border: '#4b5563',
            highlight: {
                background: '#9ca3af',
                border: '#6b7280'
            }
        },
        font: {
            color: '#ffffff',
            size: 14
        },
        shape: 'dot',
        size: node.label === currentNoteTitle ? 25 : 15
    }));
    
    console.log('[图谱] 准备边数据...');
    // 准备边数据
    const edges = data.edges.map(edge => ({
        from: edge.from,
        to: edge.to,
        arrows: 'to',
        color: {
            color: '#6b7280',
            highlight: '#4a9eff'
        },
        width: 1.5,
        smooth: {
            type: 'continuous'
        }
    }));
    
    console.log('[图谱] 创建数据集...');
    // 创建数据集
    const graphData = {
        nodes: new vis.DataSet(nodes),
        edges: new vis.DataSet(edges)
    };
    
    // 配置选项
    const options = {
        nodes: {
            borderWidth: 2,
            borderWidthSelected: 3,
            shadow: true
        },
        edges: {
            shadow: true,
            smooth: {
                type: 'continuous',
                roundness: 0.5
            }
        },
        physics: {
            enabled: true,
            stabilization: {
                enabled: true,
                iterations: 200
            },
            barnesHut: {
                gravitationalConstant: -2000,
                centralGravity: 0.3,
                springLength: 100,
                springConstant: 0.04,
                damping: 0.09,
                avoidOverlap: 0.5
            }
        },
        interaction: {
            hover: true,
            tooltipDelay: 100,
            navigationButtons: true,
            keyboard: true
        },
        layout: {
            improvedLayout: true,
            randomSeed: 42
        }
    };
    
    console.log('[图谱] 创建网络图...');
    // 创建网络图
    try {
        graphNetwork = new vis.Network(container, graphData, options);
        console.log('[图谱] 网络图创建成功');
        
        // 点击节点跳转
        graphNetwork.on('click', function(params) {
            if (params.nodes.length > 0) {
                const nodeId = params.nodes[0];
                const node = nodes.find(n => n.id === nodeId);
                if (node) {
                    console.log('[图谱] 跳转到笔记:', node.id);
                    window.location.href = `/doc/${encodeURIComponent(node.id)}`;
                }
            }
        });
        
        // 双击节点居中
        graphNetwork.on('doubleClick', function(params) {
            if (params.nodes.length > 0) {
                console.log('[图谱] 双击节点，居中:', params.nodes[0]);
                graphNetwork.focus(params.nodes[0], {
                    scale: 1.5,
                    animation: {
                        duration: 500,
                        easingFunction: 'easeInOutQuad'
                    }
                });
            }
        });
    } catch (error) {
        console.error('[图谱] 创建网络图失败:', error);
        const loading = document.getElementById('graph-loading');
        loading.innerHTML = `
            <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" style="color: var(--error-color);">
                <circle cx="12" cy="12" r="10"></circle>
                <line x1="12" y1="8" x2="12" y2="12"></line>
                <line x1="12" y1="16" x2="12.01" y2="16"></line>
            </svg>
            <p style="color: var(--error-color);">渲染图谱失败: ${error.message}</p>
        `;
        loading.style.display = 'flex';
        container.style.display = 'none';
    }
}
