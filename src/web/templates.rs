//! HTML 模板生成

/// 生成库管理页面的HTML (使用外部模板文件)
pub fn library_page() -> String {
    // 使用外部模板文件，而不是内嵌HTML
    include_str!("../../templates/library.html").to_string()
}

/// 生成库管理页面的HTML (旧版本，保留作为备份)
#[allow(dead_code)]
pub fn library_page_old() -> String {
    r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Monolith 翻译库</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; background: #f5f5f5; }
        .header { background: white; padding: 1rem; border-bottom: 1px solid #e1e5e9; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }
        .header h1 { color: #333; }
        .nav { display: flex; gap: 1rem; margin-top: 1rem; }
        .nav-btn { padding: 0.5rem 1rem; background: #667eea; color: white; border: none; border-radius: 4px; cursor: pointer; text-decoration: none; }
        .nav-btn:hover { background: #5a6fd8; }
        .container { max-width: 1200px; margin: 2rem auto; padding: 0 1rem; }
        .domain-card { background: white; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); margin-bottom: 1rem; overflow: hidden; }
        .domain-header { padding: 1rem; background: #f8f9fa; border-bottom: 1px solid #e1e5e9; display: flex; justify-content: space-between; align-items: center; }
        .domain-name { font-weight: 600; color: #333; }
        .domain-stats { color: #666; font-size: 0.9rem; }
        .domain-actions { display: flex; gap: 0.5rem; }
        .btn { padding: 0.25rem 0.75rem; border: 1px solid #ddd; border-radius: 4px; cursor: pointer; font-size: 0.8rem; }
        .btn-danger { background: #f56565; color: white; border-color: #f56565; }
        .btn-info { background: #4299e1; color: white; border-color: #4299e1; }
        .url-list { max-height: 0; overflow: hidden; transition: max-height 0.3s ease; }
        .url-list.expanded { max-height: 500px; }
        .url-item { padding: 1rem; border-bottom: 1px solid #f1f1f1; display: flex; justify-content: space-between; align-items: center; }
        .url-info { flex: 1; cursor: pointer; transition: background-color 0.2s ease; padding: 0.5rem; border-radius: 4px; margin: -0.5rem; }
        .url-info:hover { background-color: #f8f9fa; }
        .url-title { font-weight: 500; color: #333; margin-bottom: 0.25rem; }
        .url-link { color: #667eea; text-decoration: none; font-size: 0.9rem; margin-bottom: 0.25rem; display: block; }
        .url-link:hover { text-decoration: underline; }
        .url-meta { font-size: 0.8rem; color: #666; }
        .url-actions { display: flex; gap: 0.5rem; align-items: center; }
        .cache-status { display: inline-block; padding: 0.25rem 0.5rem; border-radius: 12px; font-size: 0.7rem; font-weight: 500; margin-left: 0.5rem; }
        .cache-status.permanent { background: #c6f6d5; color: #22543d; }
        .cache-status.manual { background: #e3f2fd; color: #1565c0; }
        .url-preview { display: none; position: absolute; background: white; border: 1px solid #e1e5e9; border-radius: 8px; padding: 1rem; box-shadow: 0 4px 16px rgba(0,0,0,0.1); z-index: 1000; max-width: 400px; }
        .loading { text-align: center; padding: 2rem; color: #666; }
        .error { background: #fed7d7; color: #c53030; padding: 1rem; border-radius: 4px; margin-bottom: 1rem; }
    </style>
</head>
<body>
    <header class="header">
        <h1>Monolith 翻译库</h1>
        <nav class="nav">
            <a href="/" class="nav-btn">返回首页</a>
            <button class="nav-btn" onclick="refreshLibrary()">刷新</button>
            <button class="nav-btn btn-danger" onclick="clearAllCache()">清空所有缓存</button>
        </nav>
    </header>
    
    <div class="container">
        <div class="info-banner" style="background: #e3f2fd; color: #1565c0; padding: 1rem; border-radius: 8px; margin-bottom: 1rem; font-size: 0.9rem;">
            💡 <strong>使用提示:</strong> 
            缓存已设置为永久存储，不会自动过期。点击域名右侧的"展开"按钮查看缓存的页面，点击URL可直接跳转翻译页面。
            支持快捷键：<kbd>Ctrl+R</kbd> 刷新，<kbd>Esc</kbd> 收起所有展开项。
        </div>
        
        <div id="loading" class="loading">正在加载库数据...</div>
        <div id="error" class="error" style="display: none;"></div>
        <div id="library-content"></div>
    </div>

    <script>
        let libraryData = [];
        
        async function loadLibrary() {
            try {
                document.getElementById('loading').style.display = 'block';
                document.getElementById('error').style.display = 'none';
                
                const response = await fetch('/api/library');
                if (!response.ok) throw new Error(`HTTP ${response.status}`);
                
                const data = await response.json();
                libraryData = data.domains;
                renderLibrary();
            } catch (error) {
                showError(`加载失败: ${error.message}`);
            } finally {
                document.getElementById('loading').style.display = 'none';
            }
        }
        
        function renderLibrary() {
            const content = document.getElementById('library-content');
            if (libraryData.length === 0) {
                content.innerHTML = '<div class="loading">暂无缓存数据</div>';
                return;
            }
            
            content.innerHTML = libraryData.map(domain => `
                <div class="domain-card">
                    <div class="domain-header">
                        <div>
                            <div class="domain-name">${domain.domain}</div>
                            <div class="domain-stats">
                                ${domain.cache_count} 个页面 • ${formatBytes(domain.total_size_bytes)} • 
                                最后更新: ${formatDate(domain.last_updated)}
                            </div>
                        </div>
                        <div class="domain-actions">
                            <button class="btn btn-info" onclick="toggleDomain('${domain.domain}')">
                                <span id="toggle-${domain.domain}">展开</span>
                            </button>
                            <button class="btn btn-danger" onclick="deleteDomain('${domain.domain}')">删除</button>
                        </div>
                    </div>
                    <div class="url-list" id="urls-${domain.domain}">
                        ${domain.urls.map(url => `
                            <div class="url-item">
                                <div class="url-info" onclick="openUrl('${encodeURIComponent(url.url)}')">
                                    <div class="url-title">
                                        ${url.title || '无标题'}
                                        <span class="cache-status permanent">永久缓存</span>
                                    </div>
                                    <a href="/website/${encodeURIComponent(url.url)}" class="url-link" target="_blank" onclick="event.stopPropagation()">
                                        ${truncateUrl(url.url, 60)}
                                    </a>
                                    <div class="url-meta">
                                        ${url.source_lang} → ${url.target_lang} • 
                                        ${formatDate(url.created_at)} • ${formatBytes(url.size_bytes)}
                                    </div>
                                </div>
                                <div class="url-actions">
                                    <button class="btn btn-info" onclick="openUrl('${encodeURIComponent(url.url)}', event)" title="在新标签页中翻译">翻译</button>
                                    <button class="btn btn-danger" onclick="deleteUrl('${escapeQuotes(url.url)}', '${url.source_lang}', '${url.target_lang}', event)" title="删除缓存">删除</button>
                                </div>
                            </div>
                        `).join('')}
                    </div>
                </div>
            `).join('');
        }
        
        function toggleDomain(domain) {
            const urlList = document.getElementById(`urls-${domain}`);
            const toggleBtn = document.getElementById(`toggle-${domain}`);
            
            if (urlList.classList.contains('expanded')) {
                urlList.classList.remove('expanded');
                toggleBtn.textContent = '展开';
            } else {
                urlList.classList.add('expanded');
                toggleBtn.textContent = '收起';
            }
        }
        
        async function deleteDomain(domain) {
            if (!confirm(`确定要删除域名 "${domain}" 的所有缓存吗？`)) return;
            
            try {
                const response = await fetch('/api/library/action', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ action: 'delete_domain', domain })
                });
                
                if (!response.ok) throw new Error(`HTTP ${response.status}`);
                
                const result = await response.json();
                if (result.success) {
                    await loadLibrary();
                } else {
                    showError(result.message);
                }
            } catch (error) {
                showError(`删除失败: ${error.message}`);
            }
        }
        
        function openUrl(encodedUrl, event) {
            if (event) {
                event.stopPropagation();
            }
            const websiteUrl = `/website/${encodedUrl}`;
            window.open(websiteUrl, '_blank');
        }
        
        function escapeQuotes(str) {
            return str.replace(/'/g, "\\'").replace(/"/g, '\\"');
        }
        
        async function deleteUrl(url, sourceLang, targetLang, event) {
            if (event) {
                event.stopPropagation();
            }
            if (!confirm(`确定要删除这个页面的缓存吗？`)) return;
            
            try {
                const response = await fetch('/api/library/action', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ 
                        action: 'delete_url', 
                        url, 
                        source_lang: sourceLang, 
                        target_lang: targetLang 
                    })
                });
                
                if (!response.ok) throw new Error(`HTTP ${response.status}`);
                
                const result = await response.json();
                if (result.success) {
                    await loadLibrary();
                } else {
                    showError(result.message);
                }
            } catch (error) {
                showError(`删除失败: ${error.message}`);
            }
        }
        
        async function clearAllCache() {
            if (!confirm('确定要清空所有缓存吗？这个操作不可恢复！')) return;
            
            try {
                const response = await fetch('/api/cache/clear', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({})
                });
                
                if (!response.ok) throw new Error(`HTTP ${response.status}`);
                
                const result = await response.json();
                if (result.success) {
                    await loadLibrary();
                } else {
                    showError(result.message);
                }
            } catch (error) {
                showError(`清空失败: ${error.message}`);
            }
        }
        
        function refreshLibrary() {
            loadLibrary();
        }
        
        function showError(message) {
            const errorDiv = document.getElementById('error');
            errorDiv.textContent = message;
            errorDiv.style.display = 'block';
        }
        
        function formatBytes(bytes) {
            if (bytes === 0) return '0 B';
            const k = 1024;
            const sizes = ['B', 'KB', 'MB', 'GB'];
            const i = Math.floor(Math.log(bytes) / Math.log(k));
            return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
        }
        
        function formatDate(timestamp) {
            return new Date(timestamp * 1000).toLocaleString('zh-CN');
        }
        
        // 缓存状态相关函数已移除，因为现在使用永久缓存模式
        
        function truncateUrl(url, maxLength) {
            if (url.length <= maxLength) return url;
            const start = url.substring(0, maxLength - 3);
            return start + '...';
        }
        
        // 键盘快捷键支持
        document.addEventListener('keydown', function(event) {
            // Ctrl/Cmd + R 刷新库
            if ((event.ctrlKey || event.metaKey) && event.key === 'r') {
                event.preventDefault();
                refreshLibrary();
            }
            
            // Escape 关闭所有展开的域名
            if (event.key === 'Escape') {
                document.querySelectorAll('.url-list.expanded').forEach(list => {
                    const domain = list.id.replace('urls-', '');
                    toggleDomain(domain);
                });
            }
        });
        
        // 页面加载时初始化
        document.addEventListener('DOMContentLoaded', loadLibrary);
    </script>
</body>
</html>"#.to_string()
}
