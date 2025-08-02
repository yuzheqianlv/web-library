# Core.rs 重构计划

## 概述
将现有的 core.rs 重构为更加函数式、模块化和可维护的架构。

## 1. 新的架构设计

### 1.1 核心类型系统
```rust
// 更强类型的配置系统
#[derive(Debug, Clone)]
pub struct ProcessingConfig {
    pub base_url: Option<BaseUrl>,
    pub resource_filtering: ResourceFilteringConfig,
    pub output_settings: OutputConfig,
    pub network_settings: NetworkConfig,
    pub translation_settings: Option<TranslationConfig>,
}

// 强类型的URL包装
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseUrl(Url);

// 资源过滤配置
#[derive(Debug, Clone)]
pub struct ResourceFilteringConfig {
    pub level: FilteringLevel,
    pub size_limits: SizeLimits,
    pub exclude_patterns: Vec<DomainPattern>,
    pub content_preferences: ContentPreferences,
}

// 处理结果类型
#[derive(Debug)]
pub struct ProcessedDocument {
    pub content: Vec<u8>,
    pub title: Option<String>,
    pub metadata: DocumentMetadata,
    pub statistics: ProcessingStatistics,
}
```

### 1.2 函数式处理管道
```rust
// 处理步骤特质
pub trait ProcessingStep<Input, Output> {
    type Error;
    fn process(&self, input: Input) -> Result<Output, Self::Error>;
}

// 可组合的处理器
pub struct ProcessingPipeline<T> {
    steps: Vec<Box<dyn ProcessingStep<T, T, Error = MonolithError>>>,
}

impl<T> ProcessingPipeline<T> {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }
    
    pub fn add_step<S>(mut self, step: S) -> Self 
    where
        S: ProcessingStep<T, T, Error = MonolithError> + 'static,
    {
        self.steps.push(Box::new(step));
        self
    }
    
    pub fn execute(&self, input: T) -> Result<T, MonolithError> {
        self.steps.iter().try_fold(input, |acc, step| step.process(acc))
    }
}
```

### 1.3 Builder模式配置
```rust
#[derive(Default)]
pub struct ConfigBuilder {
    config: ProcessingConfig,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn base_url(mut self, url: impl Into<BaseUrl>) -> Self {
        self.config.base_url = Some(url.into());
        self
    }
    
    pub fn disable_images(mut self) -> Self {
        self.config.resource_filtering.content_preferences.images = false;
        self
    }
    
    pub fn filtering_level(mut self, level: FilteringLevel) -> Self {
        self.config.resource_filtering.level = level;
        self
    }
    
    pub fn build(self) -> ProcessingConfig {
        self.config
    }
}
```

## 2. 模块化重构

### 2.1 核心处理模块
```
src/
├── core/
│   ├── mod.rs                  // 公共API和重新导出
│   ├── types.rs               // 核心类型定义
│   ├── config/
│   │   ├── mod.rs
│   │   ├── builder.rs         // 配置构建器
│   │   ├── validation.rs      // 配置验证
│   │   └── presets.rs         // 预设配置
│   ├── pipeline/
│   │   ├── mod.rs
│   │   ├── steps.rs           // 处理步骤定义
│   │   ├── composition.rs     // 管道组合
│   │   └── execution.rs       // 执行引擎
│   ├── processors/
│   │   ├── mod.rs
│   │   ├── encoding.rs        // 编码处理
│   │   ├── url_resolver.rs    // URL解析
│   │   ├── dom_processor.rs   // DOM处理
│   │   ├── favicon.rs         // Favicon处理
│   │   └── output.rs          // 输出格式化
│   └── utils/
│       ├── mod.rs
│       ├── media_type.rs      // 媒体类型检测
│       ├── path_formatter.rs  // 路径格式化
│       └── functional.rs      // 函数式工具
```

### 2.2 函数式工具模块
```rust
// src/core/utils/functional.rs
pub trait FunctionalExt<T> {
    fn pipe<U, F>(self, f: F) -> U
    where
        F: FnOnce(Self) -> U,
        Self: Sized;
    
    fn tap<F>(self, f: F) -> Self
    where
        F: FnOnce(&Self),
        Self: Sized;
}

impl<T> FunctionalExt<T> for T {
    fn pipe<U, F>(self, f: F) -> U
    where
        F: FnOnce(Self) -> U,
    {
        f(self)
    }
    
    fn tap<F>(self, f: F) -> Self
    where
        F: FnOnce(&Self),
    {
        f(&self);
        self
    }
}

// 结果链式操作
pub trait ResultExt<T, E> {
    fn and_tap<F>(self, f: F) -> Self
    where
        F: FnOnce(&T);
    
    fn and_tap_err<F>(self, f: F) -> Self
    where
        F: FnOnce(&E);
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn and_tap<F>(self, f: F) -> Self
    where
        F: FnOnce(&T),
    {
        if let Ok(ref value) = self {
            f(value);
        }
        self
    }
    
    fn and_tap_err<F>(self, f: F) -> Self
    where
        F: FnOnce(&E),
    {
        if let Err(ref error) = self {
            f(error);
        }
        self
    }
}
```

## 3. 具体实现示例

### 3.1 重构后的主要处理函数
```rust
pub fn create_monolithic_document_v2(
    session: Session,
    target: &str,
) -> Result<ProcessedDocument, MonolithError> {
    let config = ConfigBuilder::from_session(&session).build();
    let context = ProcessingContext::new(session, config);
    
    // 函数式管道处理
    ProcessingPipeline::new()
        .add_step(InputValidator)
        .add_step(ContentFetcher)
        .add_step(EncodingProcessor)
        .add_step(DomProcessor)
        .add_step(TranslationProcessor::conditional())
        .add_step(FaviconProcessor)
        .add_step(MetadataProcessor)
        .add_step(OutputFormatter)
        .execute(ProcessingInput::from_target(target, context))
        .map(ProcessedDocument::from)
}
```

### 3.2 可组合的处理器实现
```rust
pub struct EncodingProcessor;

impl ProcessingStep<ProcessingInput, ProcessingInput> for EncodingProcessor {
    type Error = MonolithError;
    
    fn process(&self, mut input: ProcessingInput) -> Result<ProcessingInput, Self::Error> {
        input.dom = input.raw_data
            .pipe(|data| detect_encoding(&data, input.context.config.input_encoding.as_ref()))
            .and_then(|encoding| parse_html_with_encoding(&input.raw_data, &encoding))
            .map_err(|e| MonolithError::encoding_error(e))?;
            
        Ok(input)
    }
}

pub struct DomProcessor;

impl ProcessingStep<ProcessingInput, ProcessingInput> for DomProcessor {
    type Error = MonolithError;
    
    fn process(&self, mut input: ProcessingInput) -> Result<ProcessingInput, Self::Error> {
        let base_url = resolve_base_url(&input)?;
        
        input.dom = input.dom
            .pipe(|dom| walk_and_embed_resources(dom, &mut input.context.session, &base_url))
            .map_err(MonolithError::dom_processing_error)?;
            
        Ok(input)
    }
}
```

### 3.3 错误处理改进
```rust
#[derive(Debug, thiserror::Error)]
pub enum MonolithError {
    #[error("网络错误: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("编码错误: {0}")]
    Encoding(String),
    
    #[error("DOM处理错误: {0}")]
    DomProcessing(String),
    
    #[error("URL解析错误: {0}")]
    UrlParsing(#[from] url::ParseError),
    
    #[error("配置验证错误: {0}")]
    ConfigValidation(String),
    
    #[error("翻译错误: {0}")]
    Translation(String),
}

impl MonolithError {
    pub fn encoding_error(msg: impl Into<String>) -> Self {
        Self::Encoding(msg.into())
    }
    
    pub fn dom_processing_error(msg: impl Into<String>) -> Self {
        Self::DomProcessing(msg.into())
    }
}
```

## 4. 实施步骤

### 阶段1: 类型系统重构 (第1-2周)
1. 创建新的类型定义模块
2. 实现强类型配置系统
3. 添加Builder模式支持
4. 更新错误类型系统

### 阶段2: 处理管道实现 (第3-4周)
1. 实现ProcessingStep trait
2. 创建ProcessingPipeline结构
3. 重构现有处理器为步骤
4. 添加函数式工具函数

### 阶段3: 逐步迁移 (第5-6周)
1. 保持向后兼容的API
2. 逐个迁移处理器
3. 更新测试用例
4. 性能对比和优化

### 阶段4: 清理和优化 (第7-8周)
1. 移除旧的实现
2. 完善文档和示例
3. 性能优化
4. 集成测试验证

## 5. 预期收益

### 可维护性提升
- 清晰的关注点分离
- 更容易测试的小组件
- 函数式编程减少副作用

### 可扩展性提升
- 可组合的处理管道
- 插件式的处理步骤
- 配置驱动的行为

### 代码质量提升
- 强类型系统减少运行时错误
- 函数式风格提高代码可读性
- 更好的错误处理和报告

### 性能优化潜力
- 惰性求值机会
- 更好的内存管理
- 并行处理可能性

## 6. 风险评估

### 技术风险
- 重构过程中的兼容性问题
- 性能回归的可能性
- 学习曲线对团队的影响

### 缓解策略
- 渐进式重构保持兼容性
- 全面的测试覆盖
- 详细的文档和示例
- 性能基准测试

## 结论

这个重构计划将大幅提升代码的可维护性、可扩展性和可读性，同时引入现代Rust的最佳实践。通过函数式编程模式和强类型系统，可以显著减少bug并提高开发效率。