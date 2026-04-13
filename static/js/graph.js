// ==========================================
// 关系图谱可视化逻辑 — v1.4.3
// 新增：全局图谱、节点大小/颜色、布局切换、图谱内搜索、PNG 导出
// ==========================================

let graphNetwork = null;        // 当前图谱 Network 实例
let graphDataCache = null;      // 缓存的完整图谱数据（用于筛选）
let currentGraphDepth = 2;      // 当前图谱深度（局部图谱）
let graphMode = 'local';        // 'local' | 'global'
let graphRawNodes = [];         // vis.DataSet 之前的原始节点数组
let graphRawEdges = [];         // 原始边数组

// 标签颜色调色板
const TAG_PALETTE = [
    '#7aa2f7', '#9ece6a', '#ff9e64', '#bb9af7',
    '#2ac3de', '#e0af68', '#f7768e', '#73daca',
    '#41a6b5', '#a9b1d6', '#7dcfff', '#b4f9f8',
];

// ==========================================
// 初始化：根据当前页面类型调整图谱 UI
// ==========================================
document.addEventListener('DOMContentLoaded', () => {
    const localBtn = document.getElementById('graph-mode-local');
    if (localBtn && window.currentNoteTitle) {
        // 笔记页面：显示"当前笔记"按钮并设为激活
        localBtn.style.display = '';
        localBtn.classList.add('active');
        graphMode = 'local';
        const globalBtn = document.getElementById('graph-mode-global');
        if (globalBtn) globalBtn.classList.remove('active');
    }
    // 非笔记页面："当前笔记"按钮保持隐藏（layout.html 的 style="display:none"）
});

// ==========================================
// 局部图谱（当前笔记邻域）
// ==========================================

/** 切换图谱视图显示/隐藏 */
function toggleGraphView() {
    const modal = document.getElementById('graph-modal');
    if (modal.style.display === 'none' || modal.style.display === '') {
        openGraphView();
    } else {
        closeGraphView();
    }
}

/** 打开局部图谱视图 */
function openGraphView() {
    graphMode = 'local';
    const modal = document.getElementById('graph-modal');
    modal.style.display = 'flex';
    updateGraphModeUI();
    loadGraphData(currentGraphDepth);
    document.addEventListener('keydown', handleGraphEsc);
}

/** 关闭图谱视图 */
function closeGraphView() {
    const modal = document.getElementById('graph-modal');
    modal.style.display = 'none';
    document.removeEventListener('keydown', handleGraphEsc);
    destroyGraph();
}

/** 处理 ESC 键关闭图谱 */
function handleGraphEsc(e) {
    if (e.key === 'Escape') closeGraphView();
}

/** 更新图谱深度 */
function updateGraphDepth() {
    const select = document.getElementById('graph-depth');
    currentGraphDepth = parseInt(select.value);
    if (graphMode === 'local') loadGraphData(currentGraphDepth);
}

// ==========================================
// 全局图谱
// ==========================================

/** 打开全局图谱视图 */
function openGlobalGraphView() {
    graphMode = 'global';
    const modal = document.getElementById('graph-modal');
    modal.style.display = 'flex';
    updateGraphModeUI();
    loadGlobalGraph();
    document.addEventListener('keydown', handleGraphEsc);
}

/** 切换图谱模式（局部↔全局） */
function toggleGraphMode() {
    if (graphMode === 'local') {
        graphMode = 'global';
        loadGlobalGraph();
    } else {
        graphMode = 'local';
        loadGraphData(currentGraphDepth);
    }
    updateGraphModeUI();
}

/** 更新模式切换按钮状态 */
function updateGraphModeUI() {
    const localBtn = document.getElementById('graph-mode-local');
    const globalBtn = document.getElementById('graph-mode-global');
    const depthControl = document.getElementById('graph-depth-control');
    if (localBtn) localBtn.classList.toggle('active', graphMode === 'local');
    if (globalBtn) globalBtn.classList.toggle('active', graphMode === 'global');
    if (depthControl) depthControl.style.display = graphMode === 'local' ? '' : 'none';
}

// ==========================================
// 数据加载
// ==========================================

/** 加载局部图谱数据 */
async function loadGraphData(depth) {
    const noteTitle = window.currentNoteTitle;
    if (!noteTitle) {
        showGraphError('错误：无法获取当前笔记标题');
        return;
    }
    showGraphLoading();

    try {
        const url = `/api/graph?note=${encodeURIComponent(noteTitle)}&depth=${depth}`;
        const response = await fetch(url);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const data = await response.json();
        graphDataCache = data;
        renderGraph(data, noteTitle);
    } catch (error) {
        showGraphError(`加载图谱失败: ${error.message}`);
    }
}

/** 加载全局图谱数据 */
async function loadGlobalGraph() {
    showGraphLoading();
    const hideIsolated = document.getElementById('graph-hide-isolated')?.checked ?? false;

    try {
        const url = `/api/graph/global?hide_isolated=${hideIsolated}`;
        const response = await fetch(url);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const data = await response.json();
        graphDataCache = data;
        renderGraph(data, null); // null = 无中心节点
    } catch (error) {
        showGraphError(`加载全局图谱失败: ${error.message}`);
    }
}

// ==========================================
// 图谱渲染
// ==========================================

/**
 * 渲染图谱
 * @param {Object} data - { nodes: [...], edges: [...] }
 * @param {string|null} currentNoteTitle - 中心节点标题，全局图谱时为 null
 */
function renderGraph(data, currentNoteTitle) {
    const container = document.getElementById('graph-container');

    if (typeof vis === 'undefined') {
        showGraphError('错误：可视化库 vis.js 未加载');
        return;
    }

    destroyGraph();

    // 计算每个节点的入度（反向链接数），用于节点大小
    const inDegree = {};
    data.edges.forEach(e => {
        inDegree[e.to] = (inDegree[e.to] || 0) + 1;
    });

    // 构建标签→颜色映射
    const tagColorMap = buildTagColorMap(data.nodes);

    // 准备节点数据
    const isCurrent = (label) => label === currentNoteTitle;
    graphRawNodes = data.nodes.map(node => {
        const degree = inDegree[node.id] || 0;
        const baseSize = 10 + Math.min(degree * 3, 25); // 10-35 范围
        const nodeColor = getNodeColor(node, tagColorMap, isCurrent(node.label));

        return {
            id: node.id,
            label: node.label,
            title: `${node.label}${node.tags?.length ? '\n标签: ' + node.tags.join(', ') : ''}`,
            color: nodeColor,
            font: { color: '#ffffff', size: isCurrent(node.label) ? 15 : 13 },
            shape: 'dot',
            size: isCurrent(node.label) ? 28 : baseSize,
            _tags: node.tags || [],
        };
    });

    graphRawEdges = data.edges.map(edge => ({
        from: edge.from,
        to: edge.to,
        arrows: 'to',
        color: { color: '#6b7280', highlight: '#7aa2f7' },
        width: 1.5,
        smooth: { type: 'continuous' },
    }));

    const graphData = {
        nodes: new vis.DataSet(graphRawNodes),
        edges: new vis.DataSet(graphRawEdges),
    };

    const layout = getLayoutOptions();
    const options = {
        nodes: { borderWidth: 1.5, borderWidthSelected: 2.5, shadow: true },
        edges: { shadow: false, smooth: { type: 'continuous', roundness: 0.5 } },
        physics: layout.physics,
        layout: layout.layout,
        interaction: {
            hover: true,
            tooltipDelay: 200,
            navigationButtons: false,
            keyboard: false,
        },
    };

    hideGraphLoading();
    container.style.display = 'block';

    try {
        graphNetwork = new vis.Network(container, graphData, options);

        // 点击节点跳转
        graphNetwork.on('click', function (params) {
            if (params.nodes.length > 0) {
                const nodeId = params.nodes[0];
                window.location.href = `/doc/${encodeURIComponent(nodeId)}`;
            }
        });

        // 稳定后停用物理
        graphNetwork.on('stabilizationIterationsDone', () => {
            graphNetwork.setOptions({ physics: { enabled: false } });
        });

        // 渲染标签颜色图例
        renderTagLegend(tagColorMap);

    } catch (error) {
        showGraphError(`渲染图谱失败: ${error.message}`);
    }
}

// ==========================================
// 节点颜色
// ==========================================

/** 为标签分配调色板颜色 */
function buildTagColorMap(nodes) {
    const seenTags = [];
    nodes.forEach(n => {
        if (n.tags && n.tags[0] && !seenTags.includes(n.tags[0])) {
            seenTags.push(n.tags[0]);
        }
    });
    const map = {};
    seenTags.forEach((tag, i) => {
        map[tag] = TAG_PALETTE[i % TAG_PALETTE.length];
    });
    return map;
}

/** 计算节点颜色 */
function getNodeColor(node, tagColorMap, isCurrent) {
    if (isCurrent) {
        return { background: '#4a9eff', border: '#2563eb', highlight: { background: '#3b82f6', border: '#1d4ed8' } };
    }
    const firstTag = node.tags?.[0];
    const tagColor = firstTag ? tagColorMap[firstTag] : null;
    const bg = tagColor || '#6b7280';
    return {
        background: bg,
        border: bg,
        highlight: { background: bg + 'cc', border: bg },
    };
}

// ==========================================
// 布局选项
// ==========================================

/** 获取当前布局的 vis.js 配置 */
function getLayoutOptions() {
    const mode = document.getElementById('graph-layout')?.value || 'force';
    switch (mode) {
        case 'hierarchical':
            return {
                physics: { enabled: false },
                layout: {
                    hierarchical: {
                        enabled: true,
                        direction: 'UD',
                        sortMethod: 'hubsize',
                        nodeSpacing: 120,
                        levelSeparation: 120,
                    },
                },
            };
        default: // 力导向
            return {
                physics: {
                    enabled: true,
                    stabilization: { enabled: true, iterations: 200 },
                    barnesHut: {
                        gravitationalConstant: -2000,
                        centralGravity: 0.3,
                        springLength: 100,
                        springConstant: 0.04,
                        damping: 0.09,
                        avoidOverlap: 0.5,
                    },
                },
                layout: { improvedLayout: true, randomSeed: 42 },
            };
    }
}

/** 切换布局模式 */
function changeGraphLayout() {
    if (!graphDataCache) return;
    const currentTitle = graphMode === 'local' ? window.currentNoteTitle : null;
    renderGraph(graphDataCache, currentTitle);
}

// ==========================================
// 图谱内搜索
// ==========================================

/** 在图谱中搜索并高亮定位节点 */
function searchInGraph() {
    const input = document.getElementById('graph-search-input');
    if (!input || !graphNetwork) return;
    const query = input.value.trim().toLowerCase();
    if (!query) return;

    const matched = graphRawNodes.filter(n =>
        n.label.toLowerCase().includes(query)
    );

    if (matched.length === 0) {
        input.style.borderColor = '#f7768e';
        setTimeout(() => input.style.borderColor = '', 1000);
        return;
    }

    input.style.borderColor = '#9ece6a';
    setTimeout(() => input.style.borderColor = '', 1200);

    graphNetwork.selectNodes(matched.map(n => n.id));
    graphNetwork.focus(matched[0].id, {
        scale: 1.5,
        animation: { duration: 500, easingFunction: 'easeInOutQuad' },
    });
}

// ==========================================
// 图谱导出（PNG）
// ==========================================

/** 导出当前图谱为 PNG 图片 */
function exportGraphAsPng() {
    const container = document.getElementById('graph-container');
    if (!container) return;
    const canvas = container.querySelector('canvas');
    if (!canvas) return;

    const dataUrl = canvas.toDataURL('image/png');
    const a = document.createElement('a');
    a.href = dataUrl;
    a.download = `graph-${Date.now()}.png`;
    a.click();
}

// ==========================================
// 筛选面板
// ==========================================

/** 应用筛选条件并重新渲染 */
function applyGraphFilters() {
    if (!graphDataCache) return;
    const hideIsolated = document.getElementById('graph-hide-isolated')?.checked ?? false;
    const tagFilter = (document.getElementById('graph-filter-tag')?.value || '').trim().toLowerCase();
    const folderFilter = (document.getElementById('graph-filter-folder')?.value || '').trim().toLowerCase();

    let { nodes, edges } = graphDataCache;

    if (tagFilter) {
        const matchIds = new Set(
            nodes.filter(n => (n.tags || []).some(t => t.toLowerCase().includes(tagFilter))).map(n => n.id)
        );
        nodes = nodes.filter(n => matchIds.has(n.id));
        edges = edges.filter(e => matchIds.has(e.from) && matchIds.has(e.to));
    }
    if (folderFilter) {
        nodes = nodes.filter(n => n.id.toLowerCase().includes(folderFilter));
        const ids = new Set(nodes.map(n => n.id));
        edges = edges.filter(e => ids.has(e.from) && ids.has(e.to));
    }
    if (hideIsolated) {
        const connected = new Set();
        edges.forEach(e => { connected.add(e.from); connected.add(e.to); });
        nodes = nodes.filter(n => connected.has(n.id));
    }

    const currentTitle = graphMode === 'local' ? window.currentNoteTitle : null;
    renderGraph({ nodes, edges }, currentTitle);
}

/** 重置所有筛选条件 */
function resetGraphFilters() {
    const tagInput = document.getElementById('graph-filter-tag');
    const folderInput = document.getElementById('graph-filter-folder');
    const hideIsolated = document.getElementById('graph-hide-isolated');
    if (tagInput) tagInput.value = '';
    if (folderInput) folderInput.value = '';
    if (hideIsolated) hideIsolated.checked = false;
    if (graphDataCache) {
        const currentTitle = graphMode === 'local' ? window.currentNoteTitle : null;
        renderGraph(graphDataCache, currentTitle);
    }
}

// ==========================================
// 图例
// ==========================================

/** 渲染标签颜色图例 */
function renderTagLegend(tagColorMap) {
    const legend = document.getElementById('graph-legend');
    if (!legend) return;
    const entries = Object.entries(tagColorMap);
    if (entries.length === 0) {
        legend.innerHTML = '';
        return;
    }
    legend.innerHTML = entries.slice(0, 10).map(([tag, color]) =>
        `<span class="graph-legend-item">
            <span class="graph-legend-dot" style="background:${color}"></span>${tag}
        </span>`
    ).join('');
}

// ==========================================
// 辅助函数
// ==========================================

/** 销毁当前图谱 */
function destroyGraph() {
    if (graphNetwork) {
        graphNetwork.destroy();
        graphNetwork = null;
    }
    graphRawNodes = [];
    graphRawEdges = [];
}

/** 显示加载状态 */
function showGraphLoading() {
    const container = document.getElementById('graph-container');
    const loading = document.getElementById('graph-loading');
    if (container) container.style.display = 'none';
    if (loading) {
        loading.style.display = 'flex';
        loading.innerHTML = '<div class="spinner"></div><p>加载图谱中...</p>';
    }
}

/** 隐藏加载状态 */
function hideGraphLoading() {
    const loading = document.getElementById('graph-loading');
    if (loading) loading.style.display = 'none';
}

/** 显示错误状态 */
function showGraphError(msg) {
    const loading = document.getElementById('graph-loading');
    const container = document.getElementById('graph-container');
    if (container) container.style.display = 'none';
    if (loading) {
        loading.style.display = 'flex';
        loading.innerHTML = `
            <svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" style="opacity:.5">
                <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
            </svg>
            <p style="color:var(--text-muted)">${msg}</p>`;
    }
}
