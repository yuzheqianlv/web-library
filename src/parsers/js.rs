//! JavaScript 解析器模块
//!
//! 此模块提供了用于处理 JavaScript 相关功能的工具，特别是识别和处理 DOM 事件处理器属性。
//! 在将网页保存为单一 HTML 文件时，需要正确识别哪些属性是 JavaScript 事件处理器，
//! 以便进行适当的处理。
//!
//! 主要功能：
//! - 定义了完整的 DOM 事件处理器属性列表
//! - 提供了检查属性是否为事件处理器的功能
//!
//! 该模块遵循 WHATWG HTML 规范中定义的事件处理器标准。

/// JavaScript DOM 事件处理器属性列表
///
/// 此常量包含了所有标准的 DOM 事件处理器属性名称，基于 WHATWG HTML 规范：
/// - 8.1.5.2 节："元素、Document 对象和 Window 对象上的事件处理器"
/// - 参考链接：https://html.spec.whatwg.org/#event-handlers-on-elements,-document-objects,-and-window-objects
/// - 属性列表：https://html.spec.whatwg.org/#attributes-3
///
/// 这些属性分为以下几类：
/// - 全局事件处理器：适用于大多数 HTML 元素
/// - `<body>`/`<frameset>` 特定事件处理器：仅适用于这些元素
/// - `<html>` 特定事件处理器：仅适用于根元素
const JS_DOM_EVENT_ATTRS: &[&str] = &[
    // 全局事件处理器 - 适用于大多数 HTML 元素
    "onabort",                    // 资源加载中止事件
    "onauxclick",                 // 辅助按钮点击事件（通常是中键）
    "onblur",                     // 失去焦点事件
    "oncancel",                   // 取消事件
    "oncanplay",                  // 媒体可以开始播放事件
    "oncanplaythrough",           // 媒体可以连续播放事件
    "onchange",                   // 值改变事件
    "onclick",                    // 鼠标点击事件
    "onclose",                    // 关闭事件
    "oncontextmenu",              // 右键菜单事件
    "oncuechange",                // 文本轨道提示改变事件
    "ondblclick",                 // 双击事件
    "ondrag",                     // 拖拽事件
    "ondragend",                  // 拖拽结束事件
    "ondragenter",                // 拖拽进入事件
    "ondragexit",                 // 拖拽退出事件
    "ondragleave",                // 拖拽离开事件
    "ondragover",                 // 拖拽悬停事件
    "ondragstart",                // 拖拽开始事件
    "ondrop",                     // 拖拽放下事件
    "ondurationchange",           // 媒体时长改变事件
    "onemptied",                  // 媒体清空事件
    "onended",                    // 媒体播放结束事件
    "onerror",                    // 错误事件
    "onfocus",                    // 获得焦点事件
    "onformdata",                 // 表单数据事件
    "oninput",                    // 输入事件
    "oninvalid",                  // 无效输入事件
    "onkeydown",                  // 按键按下事件
    "onkeypress",                 // 按键按压事件
    "onkeyup",                    // 按键释放事件
    "onload",                     // 加载完成事件
    "onloadeddata",               // 媒体数据加载完成事件
    "onloadedmetadata",           // 媒体元数据加载完成事件
    "onloadstart",                // 开始加载事件
    "onmousedown",                // 鼠标按下事件
    "onmouseenter",               // 鼠标进入事件
    "onmouseleave",               // 鼠标离开事件
    "onmousemove",                // 鼠标移动事件
    "onmouseout",                 // 鼠标移出事件
    "onmouseover",                // 鼠标悬停事件
    "onmouseup",                  // 鼠标释放事件
    "onwheel",                    // 鼠标滚轮事件
    "onpause",                    // 媒体暂停事件
    "onplay",                     // 媒体播放事件
    "onplaying",                  // 媒体正在播放事件
    "onprogress",                 // 进度事件
    "onratechange",               // 播放速率改变事件
    "onreset",                    // 重置事件
    "onresize",                   // 调整大小事件
    "onscroll",                   // 滚动事件
    "onsecuritypolicyviolation",  // 安全策略违规事件
    "onseeked",                   // 媒体跳转完成事件
    "onseeking",                  // 媒体跳转中事件
    "onselect",                   // 选择事件
    "onslotchange",               // 插槽改变事件
    "onstalled",                  // 媒体停滞事件
    "onsubmit",                   // 提交事件
    "onsuspend",                  // 媒体暂停事件
    "ontimeupdate",               // 时间更新事件
    "ontoggle",                   // 切换事件
    "onvolumechange",             // 音量改变事件
    "onwaiting",                  // 等待事件
    "onwebkitanimationend",       // WebKit 动画结束事件
    "onwebkitanimationiteration", // WebKit 动画迭代事件
    "onwebkitanimationstart",     // WebKit 动画开始事件
    "onwebkittransitionend",      // WebKit 过渡结束事件
    // <body/> 和 <frameset/> 元素特有的事件处理器
    "onafterprint",         // 打印后事件
    "onbeforeprint",        // 打印前事件
    "onbeforeunload",       // 页面卸载前事件
    "onhashchange",         // 哈希值改变事件
    "onlanguagechange",     // 语言改变事件
    "onmessage",            // 消息事件
    "onmessageerror",       // 消息错误事件
    "onoffline",            // 离线事件
    "ononline",             // 在线事件
    "onpagehide",           // 页面隐藏事件
    "onpageshow",           // 页面显示事件
    "onpopstate",           // 历史状态弹出事件
    "onrejectionhandled",   // Promise 拒绝处理事件
    "onstorage",            // 存储事件
    "onunhandledrejection", // 未处理的 Promise 拒绝事件
    "onunload",             // 页面卸载事件
    // <html/> 元素特有的事件处理器
    "oncut",   // 剪切事件
    "oncopy",  // 复制事件
    "onpaste", // 粘贴事件
];

/// 检查 DOM 属性名是否为原生 JavaScript 事件处理器
///
/// 此函数用于判断给定的属性名是否属于标准的 DOM 事件处理器属性。
/// 检查时不区分大小写，这符合 HTML 属性名不区分大小写的规范。
///
/// # 参数
/// * `attr_name` - 要检查的属性名称字符串切片
///
/// # 返回值
/// 如果属性名匹配任何一个标准的 DOM 事件处理器属性，返回 `true`；否则返回 `false`
///
/// # 示例
/// ```rust
/// use monolith::parsers::js::attr_is_event_handler;
///
/// assert_eq!(attr_is_event_handler("onclick"), true);
/// assert_eq!(attr_is_event_handler("OnClick"), true);  // 不区分大小写
/// assert_eq!(attr_is_event_handler("ONCLICK"), true);
/// assert_eq!(attr_is_event_handler("class"), false);   // 普通属性
/// assert_eq!(attr_is_event_handler("style"), false);   // 普通属性
/// ```
///
/// # 实现细节
/// 函数内部使用迭代器的 `any` 方法来遍历预定义的事件处理器属性列表，
/// 使用 `eq_ignore_ascii_case` 进行不区分大小写的字符串比较。
pub fn attr_is_event_handler(attr_name: &str) -> bool {
    JS_DOM_EVENT_ATTRS
        .iter()
        .any(|a| attr_name.eq_ignore_ascii_case(a))
}
