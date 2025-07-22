<details><summary>Thinking</summary>

用户正在使用基于monolith项目构建的网页翻译项目，通过MongoDB存储HTML文件。他们的要求是：在预览界面点击链接时，能够跳转到相应的翻译后的页面。

从截图可以看到：
1. 当前URL是：http://localhost:7080/website/https%3A%2F%2Fhelp.kagi.com%2Fkagi%2Fcompany%2F
2. 原始URL是：https://help.kagi.com/kagi/company/
3. 页面已经被翻译成中文
4. 页面中有很多链接（比如Vladimir Prelovac、拉古-穆尔蒂、诺曼-维纳斯基博士等）

问题是：当点击这些链接时，需要让链接指向翻译后的版本，而不是原始网站。

解决方案可能包括：
1. 在服务器端处理HTML，重写所有链接
2. 使用JavaScript在客户端拦截链接点击
3. 使用Service Worker或代理方式

我需要提供一个实际可行的解决方案。

</details>

根据您的需求，我可以提供几种实现方案来处理预览界面的链接跳转和翻译功能：

## 方案一：服务端HTML重写（推荐）

在服务器返回HTML之前，重写所有链接，使其指向您的翻译服务：

```javascript
// 处理HTML中的链接
function rewriteLinks(html, currentUrl) {
  const cheerio = require('cheerio');
  const $ = cheerio.load(html);

  // 获取当前页面的基础URL
  const baseUrl = new URL(currentUrl);

  // 处理所有链接
  $('a[href]').each((i, elem) => {
    const href = $(elem).attr('href');
    if (!href) return;

    // 处理绝对URL和相对URL
    let absoluteUrl;
    if (href.startsWith('http://') || href.startsWith('https://')) {
      absoluteUrl = href;
    } else if (href.startsWith('//')) {
      absoluteUrl = baseUrl.protocol + href;
    } else if (href.startsWith('/')) {
      absoluteUrl = baseUrl.origin + href;
    } else {
      // 相对路径
      const currentPath = baseUrl.pathname.split('/').slice(0, -1).join('/');
      absoluteUrl = baseUrl.origin + currentPath + '/' + href;
    }

    // 重写为翻译服务URL
    const encodedUrl = encodeURIComponent(absoluteUrl);
    const newHref = `/website/${encodedUrl}`;
    $(elem).attr('href', newHref);
  });

  return $.html();
}
```

## 方案二：客户端JavaScript拦截

在返回的HTML中注入JavaScript代码，拦截所有链接点击：

```javascript
// 注入到HTML页面的脚本
const linkInterceptor = `
<script>
document.addEventListener('DOMContentLoaded', function() {
  // 拦截所有链接点击
  document.addEventListener('click', function(e) {
    const link = e.target.closest('a');
    if (!link || !link.href) return;

    e.preventDefault();

    // 解析原始URL
    const url = new URL(link.href);

    // 构建翻译服务URL
    const translatedUrl = '/website/' + encodeURIComponent(url.href);

    // 跳转到翻译页面
    window.location.href = translatedUrl;
  });

  // 处理鼠标悬停显示正确的URL
  document.querySelectorAll('a[href]').forEach(link => {
    const originalHref = link.href;
    link.addEventListener('mouseenter', function() {
      const url = new URL(originalHref);
      const translatedUrl = window.location.origin + '/website/' + encodeURIComponent(url.href);
      link.setAttribute('data-original-href', originalHref);
      link.href = translatedUrl;
    });
  });
});
</script>
`;

// 在返回HTML时注入脚本
html = html.replace('</body>', linkInterceptor + '</body>');
```

## 方案三：使用MutationObserver动态处理

如果页面内容是动态加载的，可以使用MutationObserver：

```javascript
const dynamicLinkHandler = `
<script>
function processLinks(container) {
  container.querySelectorAll('a[href]').forEach(link => {
    if (link.dataset.processed) return;

    const href = link.getAttribute('href');
    if (!href) return;

    // 构建完整URL
    const absoluteUrl = new URL(href, window.location.href);

    // 标记为已处理
    link.dataset.processed = 'true';
    link.dataset.originalHref = absoluteUrl.href;

    // 点击处理
    link.addEventListener('click', function(e) {
      e.preventDefault();
      const translatedUrl = '/website/' + encodeURIComponent(this.dataset.originalHref);
      window.location.href = translatedUrl;
    });
  });
}

// 初始处理
processLinks(document.body);

// 监听DOM变化
const observer = new MutationObserver(mutations => {
  mutations.forEach(mutation => {
    mutation.addedNodes.forEach(node => {
      if (node.nodeType === 1) { // Element node
        processLinks(node);
      }
    });
  });
});

observer.observe(document.body, {
  childList: true,
  subtree: true
});
</script>
`;
```

## 方案四：Express中间件实现

如果您使用Express，可以创建一个中间件来处理：

```javascript
const express = require('express');
const cheerio = require('cheerio');

// 链接重写中间件
function linkRewriteMiddleware(req, res, next) {
  const originalSend = res.send;

  res.send = function(html) {
    if (typeof html === 'string' && html.includes('<html')) {
      const $ = cheerio.load(html);
      const decodedUrl = decodeURIComponent(req.params.url);
      const baseUrl = new URL(decodedUrl);

      // 重写所有链接
      $('a[href]').each((i, elem) => {
        const href = $(elem).attr('href');
        if (!href || href.startsWith('#')) return;

        try {
          const absoluteUrl = new URL(href, baseUrl);
          const newHref = `/website/${encodeURIComponent(absoluteUrl.href)}`;
          $(elem).attr('href', newHref);
        } catch (e) {
          console.error('链接处理错误:', e);
        }
      });

      // 处理表单action
      $('form[action]').each((i, elem) => {
        const action = $(elem).attr('action');
        if (action) {
          try {
            const absoluteUrl = new URL(action, baseUrl);
            const newAction = `/website/${encodeURIComponent(absoluteUrl.href)}`;
            $(elem).attr('action', newAction);
          } catch (e) {
            console.error('表单处理错误:', e);
          }
        }
      });

      html = $.html();
    }

    originalSend.call(this, html);
  };

  next();
}

// 使用中间件
app.get('/website/:url', linkRewriteMiddleware, async (req, res) => {
  // 您的翻译处理逻辑
});
```

## 推荐实施步骤

1. **选择方案一（服务端重写）**作为主要方案，因为它最可靠且SEO友好
2. **结合方案二（客户端拦截）**作为补充，处理动态生成的链接
3. 添加**URL基础路径处理**，确保相对路径正确解析
4. 实现**缓存机制**，避免重复翻译相同页面

这样可以确保用户在点击任何链接时都能跳转到正确的翻译页面。
