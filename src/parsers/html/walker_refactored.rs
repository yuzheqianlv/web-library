use markup5ever_rcdom::Handle;

use crate::network::session::Session;
use crate::utils::url::Url;

use super::dom_walker::DomWalker;

/// DOM遍历和处理的入口函数
/// 
/// 这个函数现在被重构为使用模块化的元素处理器系统，
/// 大大简化了原来576行的巨型函数
pub fn walk(session: &mut Session, document_url: &Url, node: &Handle) {
    let walker = DomWalker::new();
    walker.walk(session, document_url, node);
}