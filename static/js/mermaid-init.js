// Mermaid 图表初始化和主题切换
// 作者: obsidian_mirror 项目
// 功能: 初始化 Mermaid 配置，支持明暗主题切换

const MermaidManager = {
    // 初始化 Mermaid
    init() {
        // 检查 Mermaid 是否已加载
        if (typeof mermaid === 'undefined') {
            console.error('❌ Mermaid 库未加载');
            return;
        }

        // 获取当前主题
        const currentTheme = this.getCurrentTheme();
        
        // 初始化配置
        mermaid.initialize({
            startOnLoad: false, // 手动控制渲染时机
            theme: currentTheme,
            themeVariables: this.getThemeVariables(currentTheme),
            securityLevel: 'loose', // 允许点击事件
            fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, "Noto Sans", sans-serif, "Microsoft YaHei"',
            logLevel: 'error', // 只记录错误
            // 启用自动换行
            wrap: true,
            // 流程图配置
            flowchart: {
                useMaxWidth: true,
                htmlLabels: false,  // 使用纯文本渲染，边标签才能正常显示
                curve: 'basis',
                padding: 15,
                nodeSpacing: 50,
                rankSpacing: 50,
                wrappingWidth: 200
            },
            // 序列图配置
            sequence: {
                useMaxWidth: true,
                diagramMarginX: 50,
                diagramMarginY: 10,
                actorMargin: 50,
                width: 150,
                height: 65,
                boxMargin: 10,
                boxTextMargin: 5,
                noteMargin: 10,
                messageMargin: 35,
                wrap: true,
                wrapPadding: 10
            },
            // 甘特图配置
            gantt: {
                useMaxWidth: true,
                titleTopMargin: 25,
                barHeight: 20,
                barGap: 4,
                topPadding: 50,
                leftPadding: 75,
                gridLineStartPadding: 35,
                fontSize: 11,
                numberSectionStyles: 4,
                axisFormat: '%Y-%m-%d'
            },
            // 类图配置
            class: {
                useMaxWidth: true
            },
            // 状态图配置
            state: {
                useMaxWidth: true
            },
            // ER 图配置
            er: {
                useMaxWidth: true
            }
        });

        // 渲染页面中所有 Mermaid 图表
        this.renderAll();

    },

    // 获取当前主题
    getCurrentTheme() {
        // 从 html 的 data-theme 属性获取（优先级最高）
        const htmlTheme = document.documentElement.getAttribute('data-theme');
        if (htmlTheme) {
            return htmlTheme === 'dark' ? 'dark' : 'default';
        }

        // 从 localStorage 获取
        const savedTheme = localStorage.getItem('theme');
        return savedTheme === 'dark' ? 'dark' : 'default';
    },

    // 获取主题变量
    getThemeVariables(theme) {
        if (theme === 'dark') {
            // 与应用暗色主题 (Minimal 风格) 保持一致
            return {
                // 基础颜色
                darkMode: true,
                background: '#202020',
                primaryColor: '#7aa2f7',
                secondaryColor: '#2a2a2a',
                tertiaryColor: '#16161e',
                
                // 主要元素
                primaryBorderColor: '#414868',
                primaryTextColor: '#dcddde',
                secondaryTextColor: '#b3b3b3',
                
                // 节点和边
                mainBkg: '#2a2a2a',
                secondBkg: '#16161e',
                lineColor: '#6c6e70',
                border1: '#414868',
                border2: '#2a2b2e',
                
                // 文本
                textColor: '#dcddde',
                fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, "Noto Sans", sans-serif, "Microsoft YaHei"',
                fontSize: '14px',
                
                // 节点样式
                nodeBorder: '#7aa2f7',
                clusterBkg: '#1a1b1e',
                clusterBorder: '#414868',
                defaultLinkColor: '#6c6e70',
                
                // 标签
                titleColor: '#e0e0e0',
                edgeLabelBackground: 'rgba(42, 42, 42, 0.95)',
                labelBackground: '#2a2a2a',
                labelColor: '#dcddde',
                labelTextColor: '#dcddde',
                
                // 序列图
                actorBorder: '#7aa2f7',
                actorBkg: '#2a2a2a',
                actorTextColor: '#dcddde',
                actorLineColor: '#6c6e70',
                signalColor: '#dcddde',
                signalTextColor: '#dcddde',
                labelBoxBkgColor: '#1a1b1e',
                labelBoxBorderColor: '#414868',
                loopTextColor: '#dcddde',
                noteBorderColor: '#414868',
                noteBkgColor: '#16161e',
                noteTextColor: '#dcddde',
                activationBorderColor: '#7aa2f7',
                activationBkgColor: '#2a2a2a',
                sequenceNumberColor: '#dcddde',
                
                // 甘特图
                sectionBkgColor: '#2a2a2a',
                altSectionBkgColor: '#1a1b1e',
                sectionBkgColor2: '#16161e',
                taskBorderColor: '#414868',
                taskBkgColor: '#2a2a2a',
                taskTextLightColor: '#dcddde',
                taskTextColor: '#dcddde',
                taskTextDarkColor: '#dcddde',
                taskTextOutsideColor: '#dcddde',
                taskTextClickableColor: '#7aa2f7',
                activeTaskBorderColor: '#7aa2f7',
                activeTaskBkgColor: '#16161e',
                gridColor: '#414868',
                doneTaskBkgColor: '#9ece6a',
                doneTaskBorderColor: '#9ece6a',
                critBorderColor: '#f7768e',
                critBkgColor: '#5a2a2a',
                todayLineColor: '#e0af68',
                
                // 类图
                classText: '#dcddde',
                
                // 状态图
                labelColor: '#dcddde',
                
                // ER图
                attributeBackgroundColorOdd: '#2a2a2a',
                attributeBackgroundColorEven: '#1a1b1e'
            };
        } else {
            return {
                primaryColor: '#7aa2f7',
                primaryTextColor: '#333333',
                primaryBorderColor: '#e6e6e6',
                lineColor: '#666666',
                secondaryColor: '#f5f5f5',
                tertiaryColor: '#ffffff',
                background: '#fdfdfd',
                mainBkg: '#f5f5f5',
                secondBkg: '#ffffff',
                labelBackground: '#f5f5f5',
                edgeLabelBackground: 'rgba(255, 255, 255, 0.95)',
                labelColor: '#333333',
                labelTextColor: '#333333',
                nodeBorder: '#7aa2f7',
                clusterBkg: '#f7f7f7',
                clusterBorder: '#e6e6e6'
            };
        }
    },

    // 渲染所有 Mermaid 图表
    async renderAll() {
        const mermaidElements = document.querySelectorAll('.mermaid');
        
        if (mermaidElements.length === 0) {
            return; // 没有图表，不需要渲染
        }


        for (let i = 0; i < mermaidElements.length; i++) {
            const element = mermaidElements[i];
            
            // 避免重复渲染
            if (element.getAttribute('data-mermaid-rendered') === 'true') {
                continue;
            }

            try {
                // 获取图表定义（需要解码 HTML 实体）
                const rawText = element.textContent;
                let graphDefinition = this.decodeHtmlEntities(rawText);

                // 关键修复：预处理边标签中的 <br> 标签
                graphDefinition = this.fixEdgeLabelBreaks(graphDefinition);

                // v1.5.4：注入主题指令，确保与当前明/暗主题一致
                graphDefinition = this.injectThemeDirective(graphDefinition, this.getCurrentTheme());
                
                // 保存原始图表定义到 data 属性，供主题切换时使用
                element.setAttribute('data-mermaid-source', graphDefinition);
                
                // 生成唯一 ID
                const id = `mermaid-${Date.now()}-${i}`;
                
                // 渲染图表
                const { svg } = await mermaid.render(id, graphDefinition);
                
                // 替换元素内容
                element.innerHTML = svg;
                element.setAttribute('data-mermaid-rendered', 'true');
                
                // 后处理：修复节点高度问题
                this.fixNodeHeights(element);
                
            } catch (error) {
                console.error(`❌ Mermaid 图表渲染失败 (${i}):`, error);
                element.innerHTML = `<div class="mermaid-error">
                    <p>⚠️ 图表渲染失败</p>
                    <pre>${error.message}</pre>
                </div>`;
                element.setAttribute('data-mermaid-rendered', 'error');
            }
        }

    },

    // 解码 HTML 实体
    decodeHtmlEntities(text) {
        const textarea = document.createElement('textarea');
        textarea.innerHTML = text;
        return textarea.value;
    },

    // v1.5.4：在图表源码开头注入 %%{init: {...}}%% 主题指令
    // 确保每张图表都使用当前全局主题，覆盖图表自身可能携带的旧 init 指令
    injectThemeDirective(source, theme) {
        const mermaidTheme = theme === 'dark' ? 'dark' : 'default';
        // 若图表已有 %%{init}%%，跳过注入（尊重用户显式配置）
        if (/%%\s*\{/.test(source)) {
            return source;
        }
        return `%%{init: {"theme": "${mermaidTheme}"}}%%\n${source}`;
    },

    // 修复边标签中的 <br> 标签
    // 在 htmlLabels: true 模式下，边标签中的 <br> 可能无法正确渲染
    // 将 |"text <br> text"| 替换为 |"text<br/>text"| 或去掉 <br>
    fixEdgeLabelBreaks(graphDefinition) {
        // 匹配边定义中的标签部分：|"..."|  或  |...|
        // 正则：找到 | 后面跟着可选的 " ，然后是内容，然后是可选的 " 和 |
        const edgeLabelRegex = /\|(["\']?)([^|]+?)\1\|/g;
        
        return graphDefinition.replace(edgeLabelRegex, (match, quote, content) => {
            // 将 <br> 或 <br/> 或 <br /> 替换为实际的换行符
            // Mermaid 在边标签中应该会自动处理换行
            const fixed = content
                .replace(/<br\s*\/?>/gi, '\n')  // <br> → \n
                .replace(/\s+/g, ' ')            // 多个空格合并为一个
                .trim();
            
            return `|${quote}${fixed}${quote}|`;
        });
    },

    // 修复节点高度问题 - 确保多行文本完全显示
    fixNodeHeights(containerElement) {
        const svg = containerElement.querySelector('svg');
        if (!svg) return;

        // 关键修复：当 htmlLabels: false 时，需要强制修复所有 rect 的样式
        // 因为某些 CSS 规则会导致 rect 的 computed height 为 0
        const allRects = svg.querySelectorAll('rect.basic, rect.label-container, g.node rect');
        const isDark = document.documentElement.getAttribute('data-theme') === 'dark';
        
        allRects.forEach(rect => {
            // 直接用 JavaScript 设置 inline style，优先级最高
            const height = rect.getAttribute('height');
            const width = rect.getAttribute('width');
            
            if (height && width) {
                rect.style.cssText = `
                    display: block !important;
                    height: ${height}px !important;
                    width: ${width}px !important;
                    stroke: #7aa2f7 !important;
                    stroke-width: 1px !important;
                    fill: ${isDark ? '#2a2a2a' : '#ffffff'} !important;
                    opacity: 1 !important;
                    rx: 5px !important;
                    ry: 5px !important;
                `;
                
            }
        });

        // 查找所有节点
        const nodes = svg.querySelectorAll('.node');
        
        nodes.forEach(node => {
            // 获取节点中的标签容器
            const label = node.querySelector('.label');
            const foreignObject = node.querySelector('foreignObject');
            const rect = node.querySelector('rect');
            
            if (foreignObject && label) {
                // 获取标签的实际内容高度
                const labelDiv = label.querySelector('div') || label;
                const actualHeight = labelDiv.scrollHeight || labelDiv.offsetHeight;
                
                // 获取 foreignObject 的当前高度属性
                const currentHeightAttr = foreignObject.getAttribute('height');
                const currentHeight = currentHeightAttr ? parseFloat(currentHeightAttr) : NaN;
                
                // 只有当实际高度大于当前高度，或者当前高度无效时，才进行调整
                if (isNaN(currentHeight) || actualHeight > currentHeight) {
                    const newHeight = actualHeight + 20; // 额外增加一些内边距
                    foreignObject.setAttribute('height', newHeight);
                    
                    // 同时调整矩形框的高度
                    if (rect) {
                        const rectHeightAttr = rect.getAttribute('height');
                        const rectHeight = rectHeightAttr ? parseFloat(rectHeightAttr) : NaN;
                        
                        // 强制显示 rect
                        rect.style.display = 'block';
                        rect.style.visibility = 'visible';
                        rect.style.opacity = '1';
                        
                        if (!isNaN(currentHeight) && !isNaN(rectHeight)) {
                            // 两个都有效，按差值调整（保留原有的相对尺寸差异）
                            const heightDiff = newHeight - currentHeight;
                            rect.setAttribute('height', rectHeight + heightDiff);
                        } else {
                            // 其中一个无效，直接将 rect 设置为新高度（兜底策略）
                            rect.setAttribute('height', newHeight);
                        }
                    }
                }
                
                // 同样检查宽度（防止宽度过窄导致看不见）
                const actualWidth = labelDiv.scrollWidth || labelDiv.offsetWidth;
                const currentWidthAttr = foreignObject.getAttribute('width');
                const currentWidth = currentWidthAttr ? parseFloat(currentWidthAttr) : NaN;
                
                if (isNaN(currentWidth) || actualWidth > currentWidth) {
                     const newWidth = actualWidth + 20;
                     foreignObject.setAttribute('width', newWidth);
                     if (rect) {
                        const rectWidthAttr = rect.getAttribute('width');
                        const rectWidth = rectWidthAttr ? parseFloat(rectWidthAttr) : NaN;
                        
                        // 强制显示 rect
                        rect.style.display = 'block';
                        rect.style.visibility = 'visible';
                        rect.style.opacity = '1';

                        if (!isNaN(currentWidth) && !isNaN(rectWidth)) {
                            rect.setAttribute('width', rectWidth + (newWidth - currentWidth));
                        } else {
                            rect.setAttribute('width', newWidth);
                        }
                     }
                }
            }
        });

        // 修复边标签显示问题
        this.fixEdgeLabels(svg);
    },

    // 修复边标签（连线文字）显示问题
    fixEdgeLabels(svg) {
        // 查找所有边标签
        const edgeLabels = svg.querySelectorAll('g.edgeLabel');
        
        
        edgeLabels.forEach((edgeLabel, index) => {
            const foreignObject = edgeLabel.querySelector('foreignObject');
            
            if (!foreignObject) return;
            
            // 获取当前尺寸
            let currentWidth = parseFloat(foreignObject.getAttribute('width')) || 0;
            let currentHeight = parseFloat(foreignObject.getAttribute('height')) || 0;
            
            // 查找所有可能包含文本的元素
            const labelContainer = foreignObject.querySelector('div') || foreignObject.querySelector('span') || foreignObject;
            const allText = labelContainer.textContent || '';
            const trimmedText = allText.trim();
            
            
            if (!trimmedText || trimmedText.length === 0) {
                return;
            }
            
            // 核心修复：foreignObject 必须有明确的、非零的尺寸
            // 并且必须在 SVG 坐标系中正确定位
            
            // 如果尺寸太小，重新计算
            if (currentWidth < 50 || currentHeight < 20) {
                const brTags = labelContainer.querySelectorAll('br');
                const lineCount = brTags.length + 1;
                const avgCharWidth = /[\u4e00-\u9fa5]/.test(trimmedText) ? 14 : 8;
                currentWidth = Math.max(trimmedText.length * avgCharWidth / lineCount + 40, 100);
                currentHeight = Math.max(lineCount * 22 + 16, 40);
                
                foreignObject.setAttribute('width', currentWidth);
                foreignObject.setAttribute('height', currentHeight);
            }
            
            // 关键修复：移除 g.label 的 transform，直接在 foreignObject 上设置位置
            const labelG = edgeLabel.querySelector('g.label');
            if (labelG) {
                const transform = labelG.getAttribute('transform');
                
                // 解析 translate 值
                const match = transform.match(/translate\(([^,]+),\s*([^)]+)\)/);
                if (match) {
                    const tx = parseFloat(match[1]);
                    const ty = parseFloat(match[2]);
                    
                    // 将 transform 从 g.label 移到 foreignObject
                    foreignObject.setAttribute('x', tx);
                    foreignObject.setAttribute('y', ty);
                    
                    // 移除 g.label 的 transform
                    labelG.removeAttribute('transform');
                    
                }
            }
            
            // 强制设置容器样式
            if (labelContainer && labelContainer !== foreignObject) {
                const isDark = document.documentElement.getAttribute('data-theme') === 'dark';
                
                labelContainer.style.display = 'inline-block';
                labelContainer.style.visibility = 'visible';
                labelContainer.style.opacity = '1';
                labelContainer.style.padding = '6px 10px';
                labelContainer.style.backgroundColor = isDark ? 'rgba(42, 42, 42, 0.98)' : 'rgba(255, 255, 255, 0.98)';
                labelContainer.style.color = isDark ? '#dcddde' : '#333333';
                labelContainer.style.borderRadius = '4px';
                labelContainer.style.border = isDark ? '1px solid rgba(65, 72, 104, 0.6)' : '1px solid #e5e7eb';
                labelContainer.style.fontSize = '13px';
                labelContainer.style.lineHeight = '1.6';
                labelContainer.style.width = currentWidth + 'px';
                labelContainer.style.height = currentHeight + 'px';
                labelContainer.style.boxSizing = 'border-box';
                labelContainer.style.whiteSpace = 'normal';
                labelContainer.style.wordBreak = 'break-word';
                
            }
        });
    },

    // 切换主题（在用户切换明暗主题时调用）
    async switchTheme(newTheme) {

        // 重新初始化 Mermaid 配置（与 init() 方法保持一致）
        const mermaidTheme = newTheme === 'dark' ? 'dark' : 'default';
        mermaid.initialize({
            startOnLoad: false,
            theme: mermaidTheme,
            themeVariables: this.getThemeVariables(mermaidTheme),
            securityLevel: 'loose',
            fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, "Noto Sans", sans-serif, "Microsoft YaHei"',
            logLevel: 'error',
            wrap: true,
            flowchart: {
                useMaxWidth: true,
                htmlLabels: false,
                curve: 'basis',
                padding: 15,
                nodeSpacing: 50,
                rankSpacing: 50,
                wrappingWidth: 200
            },
            sequence: {
                useMaxWidth: true,
                diagramMarginX: 50,
                diagramMarginY: 10,
                actorMargin: 50,
                width: 150,
                height: 65,
                boxMargin: 10,
                boxTextMargin: 5,
                noteMargin: 10,
                messageMargin: 35,
                wrap: true,
                wrapPadding: 10
            },
            gantt: {
                useMaxWidth: true,
                titleTopMargin: 25,
                barHeight: 20,
                barGap: 4,
                topPadding: 50,
                leftPadding: 75,
                gridLineStartPadding: 35,
                fontSize: 11,
                numberSectionStyles: 4,
                axisFormat: '%Y-%m-%d'
            },
            class: {
                useMaxWidth: true
            },
            state: {
                useMaxWidth: true
            },
            er: {
                useMaxWidth: true
            }
        });

        // 重新渲染所有图表
        const mermaidElements = document.querySelectorAll('.mermaid');
        for (let i = 0; i < mermaidElements.length; i++) {
            const element = mermaidElements[i];
            
            // 跳过未渲染或错误状态的图表
            if (element.getAttribute('data-mermaid-rendered') !== 'true') {
                continue;
            }
            
            // 获取保存的原始图表定义
            let graphDefinition = element.getAttribute('data-mermaid-source');
            if (!graphDefinition) {
                continue;
            }

            // v1.5.4：重新渲染时注入新主题指令
            graphDefinition = this.injectThemeDirective(graphDefinition, newTheme === 'dark' ? 'dark' : 'default');

            try {
                // 生成唯一 ID
                const id = `mermaid-${Date.now()}-${i}`;

                // 使用新主题渲染图表
                const { svg } = await mermaid.render(id, graphDefinition);
                
                // 替换元素内容
                element.innerHTML = svg;
                
                // 后处理：修复节点高度问题
                this.fixNodeHeights(element);
                
            } catch (error) {
                console.error(`❌ Mermaid 图表重新渲染失败 (${i}):`, error);
                element.innerHTML = `<div class="mermaid-error">
                    <p>⚠️ 图表渲染失败</p>
                    <pre>${error.message}</pre>
                </div>`;
                element.setAttribute('data-mermaid-rendered', 'error');
            }
        }
    }
};

// 页面加载后初始化 Mermaid
// 注意：必须在 DOMContentLoaded 后，并且在 Mermaid 库加载完成后
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
        MermaidManager.init();
    });
} else {
    // DOM 已加载完成
    MermaidManager.init();
}

// 导出到全局，供主题切换功能调用
window.MermaidManager = MermaidManager;
