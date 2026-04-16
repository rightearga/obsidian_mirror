/* @ts-self-types="./obsidian_mirror_wasm.d.ts" */

export class NoteIndex {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(NoteIndex.prototype);
        obj.__wbg_ptr = ptr;
        NoteIndexFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        NoteIndexFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_noteindex_free(ptr, 0);
    }
    /**
     * 从服务端 index.json 的 JSON 字符串加载索引。
     *
     * index.json 格式：`[{title, path, tags, content, mtime}, ...]`
     *
     * **性能优化（v1.6.3+）**：加载时一次性分词并缓存所有字段的 token 集合，
     * 搜索时直接查询缓存，消除重复分词开销。
     * @param {string} json
     * @returns {NoteIndex}
     */
    static loadJson(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.noteindex_loadJson(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return NoteIndex.__wrap(ret[0]);
    }
    /**
     * 返回索引中的笔记总数
     * @returns {number}
     */
    noteCount() {
        const ret = wasm.noteindex_noteCount(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * 搜索笔记，返回 JSON 格式结果（与服务端 `/api/search` 响应格式一致）。
     *
     * # 评分规则
     * - 标题完全匹配每个 token：+10 分
     * - 标签匹配每个 token：+5 分
     * - 内容摘要匹配每个 token：+1 分
     *
     * # 返回格式
     * ```json
     * [{"title":"...","path":"...","snippet":"...","score":15.0,"mtime":0,"tags":["..."]}]
     * ```
     * @param {string} query
     * @param {number} limit
     * @returns {string}
     */
    searchJson(query, limit) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(query, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.noteindex_searchJson(this.__wbg_ptr, ptr0, len0, limit);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
}
if (Symbol.dispose) NoteIndex.prototype[Symbol.dispose] = NoteIndex.prototype.free;

/**
 * @param {string} nodes_json
 * @param {string} edges_json
 * @param {number} iterations
 * @returns {string}
 */
export function computeGraphLayout(nodes_json, edges_json, iterations) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(nodes_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(edges_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.computeGraphLayout(ptr0, len0, ptr1, len1, iterations);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * 计算知识地图布局（v1.9.5）
 *
 * 输入：JSON 数组 `[{id, title, path, tags, pagerank}]`（由 `/api/knowledge-map` 提供）
 *
 * 算法：
 * 1. Jaccard 相似度矩阵（共享标签数 / 并集标签数）
 * 2. 力导向布局（Fruchterman-Reingold，相似度作为吸引力权重）
 * 3. K-means 聚类（K = min(唯一标签数/3, 12)，聚类数至少 2）
 *
 * 返回：JSON 数组 `[{id, x, y, tags, cluster_id, pagerank}]`
 * @param {string} notes_json
 * @returns {string}
 */
export function computeKnowledgeMap(notes_json) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(notes_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.computeKnowledgeMap(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * 计算图谱节点的 PageRank 影响力分数（v1.9.0）
 *
 * 接受与 `computeGraphLayout` 相同格式的 JSON 输入，
 * 返回 `{node_id: score}` 格式的 JSON 对象（分数已归一化到 0.0–1.0）。
 *
 * # 参数
 * * `nodes_json`  - `[{"id": "..."},...]` 格式的节点数组
 * * `edges_json`  - `[{"from": "...", "to": "..."},...]` 格式的边数组
 * * `iterations`  - 迭代次数（建议 20，增加不显著提升精度）
 *
 * # 示例（JS）
 * ```js
 * const scores = JSON.parse(WasmLoader.computePagerank(nodesJson, edgesJson, 20));
 * // scores["folder/note.md"] → 0.75
 * ```
 * @param {string} nodes_json
 * @param {string} edges_json
 * @param {number} iterations
 * @returns {string}
 */
export function computePagerank(nodes_json, edges_json, iterations) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(nodes_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(edges_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.computePagerank(ptr0, len0, ptr1, len1, iterations);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * 本地 WASM 笔记过滤（v1.6.3）。
 *
 * 从前端缓存的 `note_items` 列表中快速过滤，支持多标签交集匹配和路径前缀过滤。
 * 与服务端搜索互补：WASM 先给出本地建议，服务端异步补充全文搜索结果。
 *
 * # 参数
 * - `notes_json`：`[{"title":"...","path":"...","tags":["..."]}]` 格式的 JSON
 * - `tags_filter`：逗号分隔的标签列表（全部匹配，OR 用多次调用实现）
 * - `folder_filter`：文件夹路径前缀（空字符串 = 不过滤）
 * - `limit`：最大返回条数
 *
 * # 返回
 * 过滤后的 `[{"title":"...","path":"...","tags":[...]}]` JSON
 * @param {string} notes_json
 * @param {string} tags_filter
 * @param {string} folder_filter
 * @param {number} limit
 * @returns {string}
 */
export function filterNotes(notes_json, tags_filter, folder_filter, limit) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(notes_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(tags_filter, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(folder_filter, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.filterNotes(ptr0, len0, ptr1, len1, ptr2, len2, limit);
        deferred4_0 = ret[0];
        deferred4_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * 从服务端渲染的 HTML 中提取目录（TOC），用于客户端快速刷新（v1.6.3）。
 *
 * 扫描 `<h1>...<h6>` 标题元素，提取 `id` 属性和文本内容，
 * 生成与服务端 `Note.toc` 格式兼容的 JSON 数组。
 *
 * 目标：100 个标题 < 1ms（替代服务端 TOC 字段，支持本地预览实时更新）。
 *
 * # 参数
 * - `html`：渲染后的 HTML 字符串（来自 `render_markdown` 或服务端）
 *
 * # 返回
 * `[{"level":2,"text":"标题","id":"anchor-id"}]` 格式的 JSON
 * @param {string} html
 * @returns {string}
 */
export function generateTocFromHtml(html) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(html, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.generateTocFromHtml(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * 在文本中将所有匹配 `term` 的位置包裹为 `<mark>...</mark>` 高亮标签。
 *
 * 大小写不敏感匹配，保留原文大小写。
 * 与服务端 `search_engine::highlight_terms` 逻辑一致，可替换其客户端等价实现。
 * @param {string} text
 * @param {string} term
 * @returns {string}
 */
export function highlight_term(text, term) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(term, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.highlight_term(ptr0, len0, ptr1, len1);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * 将 Markdown 渲染为 HTML，处理完整的 Obsidian 扩展语法（v1.6.1）。
 *
 * 处理顺序与服务端 `MarkdownProcessor::process` 保持一致：
 * 1. 预处理 `![[...]]` 图片/笔记内嵌（图片 → `<img>`，其他 → 链接）
 * 2. 预处理 `[[...]]` WikiLink（→ `/doc/...` HTML 链接）
 * 3. 预处理块级数学公式 `$$...$$`（→ `<div class="math-block">` 占位）
 * 4. 预处理行内数学公式 `$...$`（→ `<span class="math-inline">` 占位）
 * 5. 预处理高亮语法 `==text==`（→ `<mark>text</mark>`）
 * 6. pulldown-cmark 渲染（开启 Tables/Strikethrough/Tasklists/Footnotes）
 *
 * **注意**：Callout 块由客户端 `callout.js` 处理，此函数无需单独处理。
 * **注意**：不处理 YAML Frontmatter（实时预览场景通常不需要）。
 * @param {string} content
 * @returns {string}
 */
export function render_markdown(content) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(content, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.render_markdown(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * 从 HTML 中提取纯文本并截取到指定字符数（去除所有 HTML 标签）。
 *
 * 与服务端 `handlers::truncate_html` 逻辑一致，可用于客户端预览生成，
 * 减少对 `/api/preview` 接口的依赖。
 *
 * # 参数
 * - `html`：输入 HTML 字符串
 * - `max_chars`：最大可见字符数（基于 Unicode 字符，不是字节）
 * @param {string} html
 * @param {number} max_chars
 * @returns {string}
 */
export function truncate_html(html, max_chars) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(html, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.truncate_html(ptr0, len0, max_chars);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * 返回当前 WASM 模块版本（与服务端 `obsidian_mirror` 版本一致）
 *
 * 用于确认浏览器加载的 WASM 模块版本与服务端匹配。
 * @returns {string}
 */
export function wasm_version() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.wasm_version();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_throw_6b64449b9b9ed33c: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./obsidian_mirror_wasm_bg.js": import0,
    };
}

const NoteIndexFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_noteindex_free(ptr >>> 0, 1));

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasm;
function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    wasmModule = module;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('obsidian_mirror_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
