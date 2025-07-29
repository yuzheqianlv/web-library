//! 翻译批次管理器模块
//!
//! 本模块提供智能的批次分组和处理功能，用于优化大规模文本翻译的性能。
//! 通过智能算法将文本项组织成最优的批次，以提高翻译效率并减少API调用次数。
//!
//! ## 核心功能
//!
//! - **智能分组**: 根据文本优先级、复杂度和大小进行分组
//! - **批次优化**: 自动合并小批次，拆分过大批次
//! - **优先级处理**: 支持关键、高、普通、低四个优先级
//! - **并发控制**: 线程安全的批次处理和统计
//! - **队列管理**: 智能的批次队列和处理状态管理
//!
//! ## 算法优化策略
//!
//! 1. **有效大小计算**: 考虑文本复杂度，而不仅仅是字符数
//! 2. **动态批次类型**: 根据内容特征选择合适的处理策略
//! 3. **优先级排序**: 确保重要内容优先处理
//! 4. **批次合并**: 智能合并兼容的小批次以提高效率
//! 5. **超时机制**: 防止长时间阻塞的批次影响整体性能
//!
//! ## 使用示例
//!
//! ```rust
//! use crate::translation::pipeline::batch::{BatchManager, BatchManagerConfig};
//! 
//! // 创建批次管理器
//! let config = BatchManagerConfig::default();
//! let mut manager = BatchManager::new(config);
//! 
//! // 处理文本项
//! let batches = manager.create_batches(text_items);
//! 
//! // 查看统计信息
//! let stats = manager.get_stats();
//! println!("处理了 {} 个项目，生成 {} 个批次", 
//!          stats.get_input_items(), stats.get_output_batches());
//! ```

use std::collections::{HashMap, VecDeque};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

#[cfg(feature = "translation")]
use tokio::sync::Semaphore;
#[cfg(feature = "translation")]
use tokio::time::{sleep, timeout};

use crate::translation::collector::{TextItem, TextPriority};
use crate::translation::config::{constants, TranslationConfig};
use crate::translation::error::{TranslationError, TranslationResult};

/// 翻译批次信息
/// 
/// 表示一个翻译批次，包含一组待翻译的文本项和相关的处理元数据。
/// 每个批次都经过智能优化，确保在API限制内达到最佳的翻译效率。
/// 
/// ## 字段说明
/// 
/// - `id`: 唯一标识符，用于跟踪和调试
/// - `items`: 包含的文本项列表，按优先级和特征组织
/// - `priority`: 整个批次的优先级，取决于包含项目的最高优先级
/// - `batch_type`: 批次类型，影响处理策略
/// - `estimated_chars`: 预估的总字符数，用于API计费和限制检查
/// - `estimated_effective_size`: 考虑复杂度的有效大小，比字符数更准确
/// - `estimated_duration`: 预估处理时间，用于并发控制和超时设置
/// - `created_at`: 创建时间戳，用于排序和超时检测
#[derive(Debug, Clone)]
pub struct Batch {
    /// 批次唯一标识符
    pub id: usize,
    /// 包含的文本项列表
    pub items: Vec<TextItem>,
    /// 批次优先级（取所有项目的最高优先级）
    pub priority: BatchPriority,
    /// 批次类型，决定处理策略
    pub batch_type: BatchType,
    /// 预估字符总数
    pub estimated_chars: usize,
    /// 预估有效大小（考虑文本复杂度）
    pub estimated_effective_size: f32,
    /// 预估处理时间
    pub estimated_duration: Duration,
    /// 批次创建时间
    pub created_at: Instant,
}

/// 批次优先级枚举
/// 
/// 定义批次的处理优先级，影响队列顺序和资源分配。
/// 数值越大优先级越高，支持自然排序。
/// 
/// ## 优先级说明
/// 
/// - `Low`: 低优先级，适用于装饰性文本、辅助信息
/// - `Normal`: 普通优先级，适用于常规内容文本
/// - `High`: 高优先级，适用于导航、按钮等交互元素
/// - `Critical`: 关键优先级，适用于标题、重要通知等核心内容
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BatchPriority {
    /// 低优先级 - 装饰性内容
    Low = 0,
    /// 普通优先级 - 常规内容
    Normal = 1,
    /// 高优先级 - 交互元素
    High = 2,
    /// 关键优先级 - 核心内容
    Critical = 3,
}

/// 批次类型枚举
/// 
/// 根据批次大小和内容特征分类，每种类型采用不同的处理策略。
/// 
/// ## 类型说明
/// 
/// - `Standard`: 标准批次，大小适中，使用常规处理策略
/// - `Small`: 小批次，项目较少，可以考虑合并以提高效率  
/// - `Large`: 大批次，接近或超过大小限制，可能需要拆分
/// - `Single`: 单项批次，只包含一个文本项，通常是超大文本
/// - `Priority`: 优先批次，包含高优先级内容，需要优先处理
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchType {
    /// 标准批次 - 大小适中，常规处理
    Standard,
    /// 小批次 - 项目较少，可合并
    Small,
    /// 大批次 - 接近限制，可能需要拆分
    Large,
    /// 单项批次 - 只包含一个项目
    Single,
    /// 优先批次 - 高优先级内容
    Priority,
}

impl Batch {
    /// 创建新的翻译批次
    /// 
    /// 根据提供的文本项创建一个新批次，自动计算各种元数据。
    /// 
    /// # 参数
    /// 
    /// * `id` - 批次唯一标识符
    /// * `items` - 要包含在批次中的文本项列表
    /// * `batch_type` - 批次类型，影响处理策略
    /// 
    /// # 返回值
    /// 
    /// 返回包含完整元数据的新批次实例
    /// 
    /// # 自动计算
    /// 
    /// - 批次优先级（取所有项目的最高优先级）
    /// - 字符数统计
    /// - 有效大小（考虑复杂度）
    /// - 预估处理时间
    pub fn new(id: usize, items: Vec<TextItem>, batch_type: BatchType) -> Self {
        let priority = Self::calculate_batch_priority(&items);
        let estimated_chars = Self::calculate_total_chars(&items);
        let estimated_effective_size = Self::calculate_effective_size(&items);
        let estimated_duration =
            Self::estimate_duration_with_complexity(&items, estimated_effective_size);

        Self {
            id,
            items,
            priority,
            batch_type,
            estimated_chars,
            estimated_effective_size,
            estimated_duration,
            created_at: Instant::now(),
        }
    }

    /// 计算批次的整体优先级
    /// 
    /// 取所有文本项中的最高优先级作为批次的优先级。
    /// 这确保了包含重要内容的批次能够优先处理。
    /// 
    /// # 参数
    /// 
    /// * `items` - 文本项切片
    /// 
    /// # 返回值
    /// 
    /// 批次优先级，如果项目列表为空则返回 `Low`
    fn calculate_batch_priority(items: &[TextItem]) -> BatchPriority {
        if items.is_empty() {
            return BatchPriority::Low;
        }

        let max_priority = items.iter().map(|item| item.priority).max().unwrap();

        match max_priority {
            TextPriority::Critical => BatchPriority::Critical,
            TextPriority::High => BatchPriority::High,
            TextPriority::Normal => BatchPriority::Normal,
            TextPriority::Low => BatchPriority::Low,
        }
    }

    /// 计算批次中所有文本项的总字符数
    /// 
    /// 用于API计费估算和批次大小限制检查。
    /// 
    /// # 参数
    /// 
    /// * `items` - 文本项切片
    /// 
    /// # 返回值
    /// 
    /// 所有文本项的字符数总和
    fn calculate_total_chars(items: &[TextItem]) -> usize {
        items.iter().map(|item| item.char_count()).sum()
    }

    /// 计算有效大小（考虑文本复杂度）
    /// 
    /// 比简单的字符数更准确地反映翻译工作量。
    /// 考虑了文本类型、结构复杂度、HTML标签等因素。
    /// 
    /// # 参数
    /// 
    /// * `items` - 文本项切片
    /// 
    /// # 返回值
    /// 
    /// 加权后的有效大小，浮点数表示复杂度权重
    fn calculate_effective_size(items: &[TextItem]) -> f32 {
        items.iter().map(|item| item.effective_size()).sum()
    }

    /// 基于复杂度估算批次处理时间
    /// 
    /// 使用经验公式估算翻译处理时间，考虑多个因素：
    /// - 基础处理时间
    /// - 有效大小权重
    /// - 项目数量开销
    /// - 优先级影响因子
    /// 
    /// # 参数
    /// 
    /// * `items` - 文本项切片
    /// * `effective_size` - 预计算的有效大小
    /// 
    /// # 返回值
    /// 
    /// 预估的处理时间
    fn estimate_duration_with_complexity(items: &[TextItem], effective_size: f32) -> Duration {
        // 基于经验公式估算处理时间，考虑复杂度
        let base_time = Duration::from_millis(150); // 基础处理时间（API调用开销）
        let effective_time = Duration::from_millis((effective_size as u64 * 8) / 10); // 每10个有效单位8ms（复杂度加权）
        let item_time = Duration::from_millis(items.len() as u64 * 10); // 每个项目10ms（项目处理开销）

        // 考虑高优先级项目的额外处理开销
        let priority_multiplier = items
            .iter()
            .map(|item| match item.priority {
                crate::translation::collector::TextPriority::Critical => 1.3, // 关键内容需要额外质量检查
                crate::translation::collector::TextPriority::High => 1.1,      // 高优先级内容需要更仔细处理
                crate::translation::collector::TextPriority::Normal => 1.0,    // 普通内容正常处理
                crate::translation::collector::TextPriority::Low => 0.9,       // 低你先级内容可以快速处理
            })
            .fold(0.0, |acc, x| acc + x)
            / items.len() as f32; // 计算平均优先级系数

        let total_duration = base_time + effective_time + item_time;
        Duration::from_millis((total_duration.as_millis() as f32 * priority_multiplier) as u64)
    }

    /// 传统方式估算处理时间（向后兼容）
    /// 
    /// 使用简化的经验公式，不考虑文本复杂度。
    /// 保留用于兼容旧配置和测试。
    /// 
    /// # 参数
    /// 
    /// * `items` - 文本项切片
    /// * `total_chars` - 总字符数
    /// 
    /// # 返回值
    /// 
    /// 预估的处理时间
    fn estimate_duration(items: &[TextItem], total_chars: usize) -> Duration {
        // 基于经验公式估算处理时间
        let base_time = Duration::from_millis(100); // 基础时间
        let char_time = Duration::from_millis(total_chars as u64 / 10); // 每10个字符1ms
        let item_time = Duration::from_millis(items.len() as u64 * 5); // 每个项目5ms

        base_time + char_time + item_time
    }

    /// 检查两个批次是否可以合并（智能版本）
    /// 
    /// 使用有效大小和智能算法来判断合并可行性。
    /// 考虑优先级兼容性、大小限制和批次类型。
    /// 
    /// # 参数
    /// 
    /// * `other` - 要合并的另一个批次
    /// * `max_effective_size` - 最大允许的有效大小
    /// 
    /// # 返回值
    /// 
    /// 如果可以安全合并则返回 `true`
    /// 
    /// # 合并条件
    /// 
    /// 1. 优先级差异不超过 1 级
    /// 2. 合并后不超过最大有效大小
    /// 3. 不包含不兼容的批次类型
    pub fn can_merge_with(&self, other: &Batch, max_effective_size: f32) -> bool {
        // 检查优先级兼容性：只允许相邻优先级批次合并
        if (self.priority as i32 - other.priority as i32).abs() > 1 {
            return false; // 优先级相差超过 1 级，不允许合并
        }

        // 检查有效大小限制（考虑文本复杂度）
        if self.estimated_effective_size + other.estimated_effective_size > max_effective_size {
            return false; // 合并后会超过最大有效大小限制
        }

        // 检查批次类型兼容性：某些类型不允许合并
        !matches!(
            (&self.batch_type, &other.batch_type),
            (BatchType::Single, _)    // 单项批次不允许合并（通常是超大文本）
                | (_, BatchType::Single)  // 与单项批次合并
                | (BatchType::Large, _)   // 大批次不允许合并（已接近限制）
                | (_, BatchType::Large)   // 与大批次合并
        )
    }

    /// 检查两个批次是否可以合并（传统版本）
    /// 
    /// 使用简单的字符数来判断合并可行性。
    /// 保留用于向后兼容和退回处理。
    /// 
    /// # 参数
    /// 
    /// * `other` - 要合并的另一个批次
    /// * `max_size` - 最大允许的字符数
    /// 
    /// # 返回值
    /// 
    /// 如果可以安全合并则返回 `true`
    pub fn can_merge_with_legacy(&self, other: &Batch, max_size: usize) -> bool {
        // 检查优先级兼容性
        if (self.priority as i32 - other.priority as i32).abs() > 1 {
            return false;
        }

        // 检查大小限制
        if self.estimated_chars + other.estimated_chars > max_size {
            return false;
        }

        // 检查类型兼容性
        !matches!(
            (&self.batch_type, &other.batch_type),
            (BatchType::Single, _)
                | (_, BatchType::Single)
                | (BatchType::Large, _)
                | (_, BatchType::Large)
        )
    }

    /// 合并两个批次
    /// 
    /// 将另一个批次的所有内容合并到当前批次中。
    /// 自动重新计算元数据和批次类型。
    /// 
    /// # 参数
    /// 
    /// * `other` - 要合并的批次（会被消耗）
    /// 
    /// # 返回值
    /// 
    /// 合并后的新批次
    /// 
    /// # 合并逻辑
    /// 
    /// - 合并所有文本项
    /// - 取更高的优先级
    /// - 累加字符数和处理时间
    /// - 重新确定批次类型
    pub fn merge(mut self, other: Batch) -> Self {
        self.items.extend(other.items);
        self.priority = self.priority.max(other.priority);
        self.estimated_chars += other.estimated_chars;
        self.estimated_duration += other.estimated_duration;

        // 根据合并后的大小重新确定批次类型
        if self.estimated_chars > constants::MAX_BATCH_SIZE * 8 / 10 {
            self.batch_type = BatchType::Large;    // 大于 80% 限制的批次认为是大批次
        } else if self.items.len() <= constants::SMALL_BATCH_THRESHOLD {
            self.batch_type = BatchType::Small;    // 项目数较少的批次
        } else {
            self.batch_type = BatchType::Standard; // 正常大小的批次
        }

        self
    }

    /// 将过大的批次分割为多个小批次
    /// 
    /// 当批次超过最大允许大小时，将其分割为多个符合要求的小批次。
    /// 保持项目的原始顺序，确保翻译结果的一致性。
    /// 
    /// # 参数
    /// 
    /// * `max_size` - 单个批次的最大字符数
    /// 
    /// # 返回值
    /// 
    /// 分割后的批次列表，如果不需要分割则返回原批次
    /// 
    /// # 分割策略
    /// 
    /// - 逐个添加项目直到达到大小限制
    /// - 为分割后的批次分配不同的ID范围
    /// - 所有分割后的批次都使用标准类型
    pub fn split(self, max_size: usize) -> Vec<Batch> {
        if self.estimated_chars <= max_size {
            return vec![self];
        }

        let mut batches = Vec::new();
        let mut current_items = Vec::new();
        let mut current_size = 0;
        let mut batch_id = self.id;

        for item in self.items {
            let item_size = item.char_count();

            if current_size + item_size > max_size && !current_items.is_empty() {
                // 创建新批次
                batches.push(Batch::new(
                    batch_id,
                    std::mem::take(&mut current_items),
                    BatchType::Standard,
                ));
                batch_id += 1000; // 使用不同的ID范围
                current_size = 0;
            }

            current_items.push(item);
            current_size += item_size;
        }

        if !current_items.is_empty() {
            batches.push(Batch::new(batch_id, current_items, BatchType::Standard));
        }

        batches
    }

    /// 获取批次的简要描述
    /// 
    /// 生成一个人类可读的批次摘要，用于日志和调试。
    /// 
    /// # 返回值
    /// 
    /// 包含批次ID、类型、项目数、字符数和优先级的字符串
    /// 
    /// # 示例输出
    /// 
    /// `"Batch 123 (Standard): 15 items, 2048 chars, Normal priority"`
    pub fn summary(&self) -> String {
        format!(
            "Batch {} ({:?}): {} items, {} chars, {:?} priority",
            self.id,
            self.batch_type,
            self.items.len(),
            self.estimated_chars,
            self.priority
        )
    }
}

/// 批次管理器配置结构
/// 
/// 定义批次管理器的各种参数和行为开关，允许根据不同的使用场景
/// 和API限制来调整批次处理策略。
/// 
/// ## 配置类别
/// 
/// ### 大小限制
/// - 字符数限制：控制单个批次的最大字符数
/// - 有效大小限制：考虑复杂度的智能大小控制
/// - 批次阈值：定义小批次的判断标准
/// 
/// ### 性能控制
/// - 并发数量：控制同时处理的批次数量
/// - 延迟设置：批次间的处理间隔
/// - 超时控制：防止批次处理时间过长
/// 
/// ### 功能开关
/// - 智能算法：启用或禁用各种优化功能
/// - 兼容模式：支持传统处理方式
#[derive(Debug, Clone)]
pub struct BatchManagerConfig {
    /// 单个批次的最大字符数限制
    pub max_batch_size: usize,
    /// 批次的最小字符数，低于此值会尝试合并
    pub min_batch_chars: usize,
    /// 考虑复杂度的最大有效大小
    pub max_effective_size: f32,
    /// 考虑复杂度的最小有效大小
    pub min_effective_size: f32,
    /// 批次间的处理延迟时间
    pub batch_delay: Duration,
    /// 小批次的项目数阈值
    pub small_batch_threshold: usize,
    /// 是否启用按优先级排序
    pub enable_priority_sorting: bool,
    /// 是否启用批次合并优化
    pub enable_batch_merging: bool,
    /// 是否启用智能分组算法
    pub enable_smart_grouping: bool,
    /// 是否启用智能大小计算
    pub enable_smart_sizing: bool,
    /// 最大并发处理的批次数量
    pub max_concurrent_batches: usize,
    /// 单个批次的处理超时时间
    pub batch_timeout: Duration,
}

impl From<&TranslationConfig> for BatchManagerConfig {
    fn from(config: &TranslationConfig) -> Self {
        Self {
            max_batch_size: config.batch_size,
            min_batch_chars: config.min_batch_chars,
            max_effective_size: config.batch_size as f32 * 1.5,
            min_effective_size: config.min_batch_chars as f32 * 0.8,
            batch_delay: Duration::from_millis(constants::BATCH_DELAY_MS),
            small_batch_threshold: constants::SMALL_BATCH_THRESHOLD,
            batch_timeout: config.batch_timeout(),
            enable_priority_sorting: true,
            enable_batch_merging: true,
            enable_smart_grouping: true,
            enable_smart_sizing: true,
            max_concurrent_batches: config.max_concurrent_requests,
        }
    }
}

impl Default for BatchManagerConfig {
    fn default() -> Self {
        Self {
            max_batch_size: constants::MAX_BATCH_SIZE,
            min_batch_chars: constants::DEFAULT_MIN_CHARS,
            max_effective_size: constants::MAX_BATCH_SIZE as f32 * 1.5, // 允许更高的有效大小
            min_effective_size: constants::DEFAULT_MIN_CHARS as f32 * 0.8,
            batch_delay: Duration::from_millis(constants::BATCH_DELAY_MS),
            small_batch_threshold: constants::SMALL_BATCH_THRESHOLD,
            enable_priority_sorting: true,
            enable_batch_merging: true,
            enable_smart_grouping: true,
            enable_smart_sizing: true,
            max_concurrent_batches: 10,
            batch_timeout: Duration::from_secs(30),
        }
    }
}

/// 智能批次管理器（线程安全版本）
/// 
/// 负责将文本项智能地组织成最优的翻译批次，并提供并发安全的
/// 批次处理功能。使用多种算法优化批次分组，以提高翻译效率。
/// 
/// ## 主要功能
/// 
/// - **智能分组**: 根据优先级、大小和复杂度进行分组
/// - **批次优化**: 自动合并小批次，拆分大批次
/// - **并发控制**: 使用信号量控制并发批次数量
/// - **统计收集**: 收集处理统计信息用于性能分析
/// - **线程安全**: 所有操作都是线程安全的
/// 
/// ## 内部组件
/// 
/// - `config`: 配置参数，控制各种行为
/// - `stats`: 统计信息，使用Arc实现线程安全共享
/// - `next_batch_id`: 原子类型的ID生成器
/// - `semaphore`: 并发控制信号量（需要translation功能）
pub struct BatchManager {
    /// 批次管理器配置
    config: BatchManagerConfig,
    /// 处理统计信息（线程安全）
    stats: Arc<BatchStats>,
    /// 原子类型的批次ID生成器
    next_batch_id: AtomicUsize,
    /// 并发控制信号量
    #[cfg(feature = "translation")]
    semaphore: Arc<Semaphore>,
}

impl BatchManager {
    /// 创建新的批次管理器实例
    /// 
    /// 使用指定的配置创建一个新的批次管理器。
    /// 初始化所有必要的线程安全组件。
    /// 
    /// # 参数
    /// 
    /// * `config` - 批次管理器配置
    /// 
    /// # 返回值
    /// 
    /// 新的批次管理器实例
    pub fn new(config: BatchManagerConfig) -> Self {
        #[cfg(feature = "translation")]
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_batches));

        Self {
            config,
            stats: Arc::new(BatchStats::default()),
            next_batch_id: AtomicUsize::new(1),
            #[cfg(feature = "translation")]
            semaphore,
        }
    }

    /// 使用默认配置创建批次管理器
    /// 
    /// 便捷方法，使用合理的默认参数创建管理器实例。
    /// 适用于大多数常规使用场景。
    /// 
    /// # 返回值
    /// 
    /// 使用默认配置的批次管理器实例
    pub fn default() -> Self {
        Self::new(BatchManagerConfig::default())
    }

    /// 创建优化的翻译批次
    /// 
    /// 主要的批次创建接口，将文本项列表转换为优化的批次列表。
    /// 使用多种算法进行智能优化。
    /// 
    /// # 参数
    /// 
    /// * `items` - 待处理的文本项列表（会被修改用于排序）
    /// 
    /// # 返回值
    /// 
    /// 优化后的批次列表，按优先级排序
    /// 
    /// # 处理流程
    /// 
    /// 1. 统计重置和输入计数
    /// 2. 优先级排序（如果启用）
    /// 3. 智能或简单分组
    /// 4. 批次合并优化（如果启用）
    /// 5. 更新统计信息
    pub fn create_batches(&mut self, mut items: Vec<TextItem>) -> Vec<Batch> {
        let start_time = Instant::now();
        self.stats.reset();
        self.stats.add_input_items(items.len());

        if items.is_empty() {
            return Vec::new();
        }

        // 排序优化
        if self.config.enable_priority_sorting {
            self.sort_items_optimally(&mut items);
        }

        // 智能分组
        let batches = if self.config.enable_smart_grouping {
            self.create_smart_batches(items)
        } else {
            self.create_simple_batches(items)
        };

        // 批次合并优化
        let optimized_batches = if self.config.enable_batch_merging {
            self.optimize_batches(batches)
        } else {
            batches
        };

        self.stats
            .output_batches
            .store(optimized_batches.len(), Ordering::Relaxed);
        self.stats.add_processing_time(start_time.elapsed());

        optimized_batches
    }

    /// 文本项的最优排序算法
    /// 
    /// 使用多级排序策略来优化翻译效率：
    /// 1. 伖先级高的在前（确保重要内容优先处理）
    /// 2. 短文本在前（提高批次填充效率）
    /// 3. 浅层节点在前（简化上下文处理）
    /// 
    /// # 参数
    /// 
    /// * `items` - 要排序的文本项列表（原地修改）
    fn sort_items_optimally(&self, items: &mut Vec<TextItem>) {
        items.sort_by(|a, b| {
            // 多级排序策略
            b.priority
                .cmp(&a.priority) // 优先级高的在前
                .then_with(|| a.char_count().cmp(&b.char_count())) // 短文本在前
                .then_with(|| a.depth.cmp(&b.depth)) // 浅层节点在前
        });
    }

    /// 智能批次创建算法
    /// 
    /// 使用高级算法创建最优批次，包括：
    /// - 按优先级分组处理
    /// - 为每个优先级创建专门的批次
    /// - 保证高优先级内容优先处理
    /// 
    /// # 参数
    /// 
    /// * `items` - 已排序的文本项列表
    /// 
    /// # 返回值
    /// 
    /// 按优先级组织的批次列表
    fn create_smart_batches(&mut self, items: Vec<TextItem>) -> Vec<Batch> {
        let mut batches = Vec::new();
        let mut priority_groups = self.group_by_priority(items);

        // 按优先级处理
        for priority in [
            BatchPriority::Critical,
            BatchPriority::High,
            BatchPriority::Normal,
            BatchPriority::Low,
        ] {
            if let Some(items) = priority_groups.remove(&priority) {
                let mut priority_batches = self.create_batches_for_priority(items, priority);
                batches.append(&mut priority_batches);
            }
        }

        batches
    }

    /// 按优先级将文本项分组
    /// 
    /// 将文本项按照优先级分类，为后续的分组处理做准备。
    /// 
    /// # 参数
    /// 
    /// * `items` - 要分组的文本项列表
    /// 
    /// # 返回值
    /// 
    /// 以优先级为键的文本项分组映射
    fn group_by_priority(&self, items: Vec<TextItem>) -> HashMap<BatchPriority, Vec<TextItem>> {
        let mut groups: HashMap<BatchPriority, Vec<TextItem>> = HashMap::new();

        for item in items {
            let batch_priority = match item.priority {
                TextPriority::Critical => BatchPriority::Critical,
                TextPriority::High => BatchPriority::High,
                TextPriority::Normal => BatchPriority::Normal,
                TextPriority::Low => BatchPriority::Low,
            };
            groups.entry(batch_priority).or_default().push(item);
        }

        groups
    }

    /// 为特定优先级创建批次
    /// 
    /// 根据配置选择使用智能或传统算法来创建批次。
    /// 
    /// # 参数
    /// 
    /// * `items` - 特定优先级的文本项列表
    /// * `_priority` - 优先级参数（留作未来使用）
    /// 
    /// # 返回值
    /// 
    /// 优化后的批次列表
    fn create_batches_for_priority(
        &mut self,
        items: Vec<TextItem>,
        _priority: BatchPriority,
    ) -> Vec<Batch> {
        if self.config.enable_smart_sizing {
            self.create_smart_batches_for_priority(items)
        } else {
            self.create_legacy_batches_for_priority(items)
        }
    }

    /// 使用有效大小的智能批次创建
    /// 
    /// 基于文本的有效大小（考虑复杂度）来创建批次，
    /// 比简单的字符数更准确地反映翻译工作量。
    /// 
    /// # 参数
    /// 
    /// * `items` - 要分组的文本项列表
    /// 
    /// # 返回值
    /// 
    /// 基于有效大小优化的批次列表
    /// 
    /// # 算法特点
    /// 
    /// - 同时检查有效大小和字符数限制
    /// - 动态确定批次类型
    /// - 优化批次大小分布
    fn create_smart_batches_for_priority(&mut self, items: Vec<TextItem>) -> Vec<Batch> {
        let mut batches = Vec::new();
        let mut current_batch = Vec::new();
        let mut current_effective_size = 0.0;
        let mut current_char_size = 0;

        for item in items {
            let item_effective_size = item.effective_size();
            let item_char_size = item.char_count();

            // 检查是否需要创建新批次（双重限制检查）
            let should_create_new_batch = {
                !current_batch.is_empty()  // 当前批次不为空
                    && (current_effective_size + item_effective_size
                        > self.config.max_effective_size         // 超过有效大小限制
                        || current_char_size + item_char_size > self.config.max_batch_size) // 或超过字符数限制
            };

            if should_create_new_batch {
                // 完成当前批次并创建新批次
                if !current_batch.is_empty() {
                    let batch_type = self.determine_batch_type(
                        current_batch.len(),
                        current_char_size,
                        current_effective_size,
                    );
                    let batch = Batch::new(
                        self.next_batch_id.fetch_add(1, Ordering::Relaxed), // 原子操作生成唯一ID
                        std::mem::take(&mut current_batch),                  // 移动语义转移所有权
                        batch_type,
                    );
                    batches.push(batch);
                    current_effective_size = 0.0;  // 重置统计信息
                    current_char_size = 0;
                }
            }

            // 将项目添加到当前批次并更新统计信息
            current_batch.push(item);
            current_effective_size += item_effective_size; // 累加有效大小
            current_char_size += item_char_size;           // 累加字符数
        }

        // 处理最后一个批次
        if !current_batch.is_empty() {
            let batch_type = self.determine_batch_type(
                current_batch.len(),
                current_char_size,
                current_effective_size,
            );
            let batch = Batch::new(
                self.next_batch_id.fetch_add(1, Ordering::Relaxed),
                current_batch,
                batch_type,
            );
            batches.push(batch);
        }

        batches
    }

    /// 传统批次创建算法（向后兼容）
    /// 
    /// 使用简单的字符数算法创建批次，保留用于：
    /// - 向后兼容旧版本
    /// - 简单配置下的退回处理
    /// - 性能对比测试
    /// 
    /// # 参数
    /// 
    /// * `items` - 要分组的文本项列表
    /// 
    /// # 返回值
    /// 
    /// 使用传统算法的批次列表
    fn create_legacy_batches_for_priority(&mut self, items: Vec<TextItem>) -> Vec<Batch> {
        let mut batches = Vec::new();
        let mut current_batch = Vec::new();
        let mut current_size = 0;

        for item in items {
            let item_size = item.char_count();

            if !current_batch.is_empty() && current_size + item_size > self.config.max_batch_size {
                // 创建批次
                let batch_type = if current_batch.len() == 1 {
                    BatchType::Single
                } else if current_batch.len() <= self.config.small_batch_threshold {
                    BatchType::Small
                } else {
                    BatchType::Standard
                };

                let batch = Batch::new(
                    self.next_batch_id.fetch_add(1, Ordering::Relaxed),
                    std::mem::take(&mut current_batch),
                    batch_type,
                );
                batches.push(batch);
                current_size = 0;
            }

            current_batch.push(item);
            current_size += item_size;
        }

        // 处理最后一个批次
        if !current_batch.is_empty() {
            let batch_type = if current_batch.len() == 1 {
                BatchType::Single
            } else if current_batch.len() <= self.config.small_batch_threshold {
                BatchType::Small
            } else {
                BatchType::Standard
            };

            let batch = Batch::new(
                self.next_batch_id.fetch_add(1, Ordering::Relaxed),
                current_batch,
                batch_type,
            );
            batches.push(batch);
        }

        batches
    }

    /// 智能确定批次类型
    /// 
    /// 根据多个指标智能判断批次类型：
    /// - 项目数量、字符数、有效大小
    /// - 与配置限制的比较
    /// 
    /// # 参数
    /// 
    /// * `item_count` - 批次中的项目数量
    /// * `char_size` - 批次的字符数
    /// * `effective_size` - 批次的有效大小
    /// 
    /// # 返回值
    /// 
    /// 适合的批次类型
    fn determine_batch_type(
        &self,
        item_count: usize,
        char_size: usize,
        effective_size: f32,
    ) -> BatchType {
        if item_count == 1 {
            BatchType::Single
        } else if item_count <= self.config.small_batch_threshold {
            BatchType::Small
        } else if char_size > self.config.max_batch_size * 8 / 10
            || effective_size > self.config.max_effective_size * 0.9
        {
            BatchType::Large
        } else {
            BatchType::Standard
        }
    }

    /// 简单批次创建算法（向后兼容）
    /// 
    /// 不使用优先级分组，直接按顺序创建批次。
    /// 主要用于简单场景和性能对比。
    /// 
    /// # 参数
    /// 
    /// * `items` - 要分组的文本项列表
    /// 
    /// # 返回值
    /// 
    /// 简单分组的批次列表
    fn create_simple_batches(&mut self, items: Vec<TextItem>) -> Vec<Batch> {
        let mut batches = Vec::new();
        let mut current_batch = Vec::new();
        let mut current_size = 0;

        for item in items {
            let item_size = item.char_count();

            if item_size > self.config.max_batch_size {
                self.finalize_current_batch(&mut batches, &mut current_batch, current_size);
                batches.push(Batch::new(
                    self.next_batch_id(),
                    vec![item],
                    BatchType::Single,
                ));
                current_size = 0;
                continue;
            }

            if self.should_start_new_batch(current_size, item_size, &current_batch) {
                self.finalize_current_batch(&mut batches, &mut current_batch, current_size);
                current_size = 0;
            }

            current_batch.push(item);
            current_size += item_size;
        }

        if !current_batch.is_empty() {
            self.finalize_current_batch(&mut batches, &mut current_batch, current_size);
        }

        batches
    }

    /// 判断是否应该开始新批次（智能版本）
    fn should_start_new_smart_batch(
        &self,
        current_effective_size: f32,
        item_effective_size: f32,
        current_char_size: usize,
        item_char_size: usize,
        current_batch: &[TextItem],
    ) -> bool {
        if current_batch.is_empty() {
            return false;
        }

        // 检查有效大小限制
        let would_exceed_effective =
            current_effective_size + item_effective_size > self.config.max_effective_size;
        // 检查字符数限制（作为后备）
        let would_exceed_chars = current_char_size + item_char_size > self.config.max_batch_size;

        let has_min_effective = current_effective_size >= self.config.min_effective_size;
        let has_min_chars = current_char_size >= self.config.min_batch_chars;
        let is_near_limit = current_effective_size > self.config.max_effective_size * 0.8;

        (would_exceed_effective || would_exceed_chars)
            && (has_min_effective || has_min_chars || is_near_limit)
    }

    /// 判断是否应该开始新批次（传统版本）
    fn should_start_new_batch(
        &self,
        current_size: usize,
        item_size: usize,
        current_batch: &[TextItem],
    ) -> bool {
        !current_batch.is_empty()
            && (current_size + item_size > self.config.max_batch_size)
            && (current_size >= self.config.min_batch_chars
                || current_size > self.config.max_batch_size * 8 / 10)
    }

    /// 完成当前批次（智能版本）
    fn finalize_current_smart_batch(
        &mut self,
        batches: &mut Vec<Batch>,
        current_batch: &mut Vec<TextItem>,
        current_effective_size: f32,
        current_char_size: usize,
    ) {
        if current_batch.is_empty() {
            return;
        }

        let batch_type = if current_batch.len() <= self.config.small_batch_threshold {
            BatchType::Small
        } else if current_effective_size > self.config.max_effective_size * 0.8 {
            BatchType::Large
        } else {
            BatchType::Standard
        };

        let batch = Batch::new(
            self.next_batch_id(),
            std::mem::take(current_batch),
            batch_type,
        );

        batches.push(batch);

        // 智能警告
        if current_effective_size < self.config.min_effective_size {
            tracing::debug!(
                "批次有效大小 {:.1} 未达到最小要求 {:.1}",
                current_effective_size,
                self.config.min_effective_size
            );
        }

        if current_char_size < self.config.min_batch_chars {
            tracing::debug!(
                "批次字符数 {} 未达到最小要求 {}",
                current_char_size,
                self.config.min_batch_chars
            );
        }
    }

    /// 完成当前批次（传统版本）
    fn finalize_current_batch(
        &mut self,
        batches: &mut Vec<Batch>,
        current_batch: &mut Vec<TextItem>,
        current_size: usize,
    ) {
        if current_batch.is_empty() {
            return;
        }

        let batch_type = if current_batch.len() <= self.config.small_batch_threshold {
            BatchType::Small
        } else if current_size > self.config.max_batch_size * 8 / 10 {
            BatchType::Large
        } else {
            BatchType::Standard
        };

        let batch = Batch::new(
            self.next_batch_id(),
            std::mem::take(current_batch),
            batch_type,
        );

        batches.push(batch);

        if current_size < self.config.min_batch_chars {
            tracing::debug!(
                "批次字符数 {} 未达到最小要求 {}",
                current_size,
                self.config.min_batch_chars
            );
        }
    }

    /// 优化批次（智能合并）
    fn optimize_batches(&mut self, batches: Vec<Batch>) -> Vec<Batch> {
        if batches.len() <= 1 {
            return batches;
        }

        if self.config.enable_smart_sizing {
            self.optimize_batches_smart(batches)
        } else {
            self.optimize_batches_legacy(batches)
        }
    }

    /// 智能批次优化
    fn optimize_batches_smart(&mut self, batches: Vec<Batch>) -> Vec<Batch> {
        let mut optimized = Vec::new();
        let mut merge_candidates = VecDeque::new();

        // 将小批次和标准批次分开处理
        for batch in batches {
            match batch.batch_type {
                BatchType::Small => merge_candidates.push_back(batch),
                _ => {
                    // 先处理合并候选
                    if !merge_candidates.is_empty() {
                        optimized.extend(self.merge_small_batches_smart(&mut merge_candidates));
                    }
                    optimized.push(batch);
                }
            }
        }

        // 处理剩余的合并候选
        if !merge_candidates.is_empty() {
            optimized.extend(self.merge_small_batches_smart(&mut merge_candidates));
        }

        optimized
    }

    /// 传统批次优化
    fn optimize_batches_legacy(&mut self, batches: Vec<Batch>) -> Vec<Batch> {
        let mut optimized = Vec::new();
        let mut merge_candidates = VecDeque::new();

        // 将小批次和标准批次分开处理
        for batch in batches {
            match batch.batch_type {
                BatchType::Small => merge_candidates.push_back(batch),
                _ => {
                    // 先处理合并候选
                    if !merge_candidates.is_empty() {
                        optimized.extend(self.merge_small_batches_smart(&mut merge_candidates));
                    }
                    optimized.push(batch);
                }
            }
        }

        // 处理剩余的合并候选
        if !merge_candidates.is_empty() {
            optimized.extend(self.merge_small_batches_smart(&mut merge_candidates));
        }

        optimized
    }

    /// 智能合并小批次
    fn merge_small_batches_smart(&mut self, candidates: &mut VecDeque<Batch>) -> Vec<Batch> {
        let mut merged = Vec::new();

        while let Some(mut current) = candidates.pop_front() {
            // 尝试与后续批次合并（使用有效大小）
            while let Some(next) = candidates.front() {
                if current.can_merge_with(next, self.config.max_effective_size) {
                    let next_batch = candidates.pop_front().unwrap();
                    current = current.merge(next_batch);
                    self.stats.inc_merged_batches();
                } else {
                    break;
                }
            }
            merged.push(current);
        }

        merged
    }

    /// 传统合并小批次
    fn merge_small_batches_legacy(&mut self, candidates: &mut VecDeque<Batch>) -> Vec<Batch> {
        let mut merged = Vec::new();

        while let Some(mut current) = candidates.pop_front() {
            // 尝试与后续批次合并
            while let Some(next) = candidates.front() {
                if current.can_merge_with_legacy(next, self.config.max_batch_size) {
                    let next_batch = candidates.pop_front().unwrap();
                    current = current.merge(next_batch);
                    self.stats.inc_merged_batches();
                } else {
                    break;
                }
            }
            merged.push(current);
        }

        merged
    }

    /// 获取下一个批次ID（线程安全）
    fn next_batch_id(&self) -> usize {
        self.next_batch_id.fetch_add(1, Ordering::Relaxed)
    }

    /// 并行处理批次（修改为顺序处理以避免Send问题）
    #[cfg(feature = "translation")]
    pub async fn process_batches_sequential<F, Fut>(
        &self,
        batches: Vec<Batch>,
        mut processor: F,
    ) -> TranslationResult<()>
    where
        F: FnMut(Batch) -> Fut + Send,
        Fut: std::future::Future<Output = TranslationResult<()>> + Send,
    {
        for batch in batches {
            // 获取信号量许可
            let _permit = self.semaphore.acquire().await.map_err(|e| {
                TranslationError::ConcurrencyError(format!("获取信号量失败: {}", e))
            })?;

            // 批次间延迟
            if batch.id > 1 {
                sleep(Duration::from_millis(constants::BATCH_DELAY_MS)).await;
            }

            // 执行处理（带超时）
            timeout(self.config.batch_timeout, processor(batch))
                .await
                .map_err(|_| TranslationError::TimeoutError("批次处理超时".to_string()))??;
        }

        Ok(())
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> &BatchStats {
        &self.stats
    }

    /// 重置统计信息
    pub fn reset_stats(&mut self) {
        self.stats.reset();
    }
}

/// 批次处理统计信息（线程安全版本）
/// 
/// 收集和维护批次处理过程中的各种统计数据，用于性能分析、
/// 监控和优化。所有字段都使用原子类型确保线程安全。
/// 
/// ## 统计指标
/// 
/// ### 基础统计
/// - 输入项目数量：原始文本项的总数
/// - 输出批次数量：生成的批次总数
/// - 处理时间：批次创建的总耗时
/// 
/// ### 优化统计
/// - 合并批次数：通过合并优化减少的批次数
/// - 拆分批次数：因大小限制拆分的批次数
/// 
/// ## 线程安全
/// 
/// 所有操作都使用原子操作，可以在多线程环境中安全使用。
/// 使用 `Relaxed` 内存序，适合统计场景的性能要求。
#[derive(Debug, Default)]
pub struct BatchStats {
    /// 输入的文本项总数
    pub input_items: AtomicUsize,
    /// 输出的批次总数
    pub output_batches: AtomicUsize,
    /// 合并优化的批次数量
    pub merged_batches: AtomicUsize,
    /// 拆分处理的批次数量
    pub split_batches: AtomicUsize,
    /// 总处理时间（微秒）
    pub processing_time: std::sync::atomic::AtomicU64,
    // 注意：更复杂的统计（如平均值和分布）需要额外的同步机制
    // 这里为了简化和性能考虑，暂时省略
    // pub average_batch_size: AtomicU32,
    // pub batch_size_distribution: DashMap<usize, AtomicUsize>,
}

impl BatchStats {
    /// 增加输入项数
    pub fn add_input_items(&self, count: usize) {
        self.input_items.fetch_add(count, Ordering::Relaxed);
    }

    /// 增加输出批次数
    pub fn inc_output_batches(&self) {
        self.output_batches.fetch_add(1, Ordering::Relaxed);
    }

    /// 增加合并批次数
    pub fn inc_merged_batches(&self) {
        self.merged_batches.fetch_add(1, Ordering::Relaxed);
    }

    /// 增加拆分批次数
    pub fn inc_split_batches(&self) {
        self.split_batches.fetch_add(1, Ordering::Relaxed);
    }

    /// 添加处理时间
    pub fn add_processing_time(&self, duration: Duration) {
        self.processing_time
            .fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }

    /// 重置统计（仅重置可以安全重置的值）
    pub fn reset(&self) {
        self.input_items.store(0, Ordering::Relaxed);
        self.output_batches.store(0, Ordering::Relaxed);
        self.merged_batches.store(0, Ordering::Relaxed);
        self.split_batches.store(0, Ordering::Relaxed);
        self.processing_time.store(0, Ordering::Relaxed);
    }
}

impl BatchStats {
    /// 计算平均批次大小
    pub fn calculate_average_size(&self) -> f32 {
        let input_items = self.input_items.load(Ordering::Relaxed);
        let output_batches = self.output_batches.load(Ordering::Relaxed);
        if output_batches == 0 {
            0.0
        } else {
            input_items as f32 / output_batches as f32
        }
    }

    /// 获取批次效率
    pub fn batch_efficiency(&self) -> f32 {
        let input_items = self.input_items.load(Ordering::Relaxed);
        if input_items == 0 {
            0.0
        } else {
            let optimal_batches = (input_items as f32 / constants::MAX_BATCH_SIZE as f32).ceil();
            let output_batches = self.output_batches.load(Ordering::Relaxed);
            optimal_batches / output_batches as f32
        }
    }

    /// 获取输入项数
    pub fn get_input_items(&self) -> usize {
        self.input_items.load(Ordering::Relaxed)
    }

    /// 获取输出批次数
    pub fn get_output_batches(&self) -> usize {
        self.output_batches.load(Ordering::Relaxed)
    }
}

/// 批次队列管理器
/// 
/// 提供基于优先级的批次队列管理功能。支持多个优先级队列，
/// 确保高优先级批次优先处理，并提供超时检测功能。
/// 
/// ## 队列组织
/// 
/// - **高优先级队列**: 处理 Critical 和 High 优先级批次
/// - **普通优先级队列**: 处理 Normal 优先级批次
/// - **低优先级队列**: 处理 Low 优先级批次
/// 
/// ## 处理逻辑
/// 
/// 1. 优先从高优先级队列取批次
/// 2. 高优先级队列为空时，再从普通优先级队列取
/// 3. 最后从低优先级队列取
/// 4. 跟踪正在处理的批次及其起始时间
pub struct BatchQueue {
    /// 高优先级批次队列（Critical + High）
    high_priority: VecDeque<Batch>,
    /// 普通优先级批次队列（Normal）
    normal_priority: VecDeque<Batch>,
    /// 低优先级批次队列（Low）
    low_priority: VecDeque<Batch>,
    /// 正在处理的批次及其开始时间
    processing: HashMap<usize, Instant>,
}

impl BatchQueue {
    /// 创建新的空批次队列
    /// 
    /// 初始化所有优先级队列和处理状态追踪。
    /// 
    /// # 返回值
    /// 
    /// 新的空批次队列实例
    pub fn new() -> Self {
        Self {
            high_priority: VecDeque::new(),
            normal_priority: VecDeque::new(),
            low_priority: VecDeque::new(),
            processing: HashMap::new(),
        }
    }

    /// 将批次添加到相应的优先级队列
    /// 
    /// 根据批次的优先级自动选择合适的队列。
    /// 
    /// # 参数
    /// 
    /// * `batch` - 要添加的批次
    /// 
    /// # 队列选择
    /// 
    /// - Critical/High 优先级 → 高优先级队列
    /// - Normal 优先级 → 普通优先级队列
    /// - Low 优先级 → 低优先级队列
    pub fn enqueue(&mut self, batch: Batch) {
        match batch.priority {
            BatchPriority::Critical | BatchPriority::High => {
                self.high_priority.push_back(batch);
            }
            BatchPriority::Normal => {
                self.normal_priority.push_back(batch);
            }
            BatchPriority::Low => {
                self.low_priority.push_back(batch);
            }
        }
    }

    /// 从队列中取出下一个要处理的批次
    /// 
    /// 按照优先级顺序取出批次，高优先级优先。
    /// 同时将批次标记为“正在处理”状态。
    /// 
    /// # 返回值
    /// 
    /// - `Some(Batch)`: 下一个要处理的批次
    /// - `None`: 所有队列都为空
    /// 
    /// # 处理顺序
    /// 
    /// 1. 高优先级队列
    /// 2. 普通优先级队列 
    /// 3. 低优先级队列
    pub fn dequeue(&mut self) -> Option<Batch> {
        // 优先处理高优先级批次
        if let Some(batch) = self.high_priority.pop_front() {
            self.processing.insert(batch.id, Instant::now());
            Some(batch)
        } else if let Some(batch) = self.normal_priority.pop_front() {
            self.processing.insert(batch.id, Instant::now());
            Some(batch)
        } else if let Some(batch) = self.low_priority.pop_front() {
            self.processing.insert(batch.id, Instant::now());
            Some(batch)
        } else {
            None
        }
    }

    /// 标记批次处理完成
    /// 
    /// 从正在处理的批次列表中移除指定批次。
    /// 
    /// # 参数
    /// 
    /// * `batch_id` - 已完成处理的批次ID
    pub fn complete(&mut self, batch_id: usize) {
        self.processing.remove(&batch_id);
    }

    /// 获取当前队列的状态信息
    /// 
    /// 返回各个优先级队列的批次数量和处理状态。
    /// 
    /// # 返回值
    /// 
    /// 包含详细队列状态的 `QueueStatus` 结构体
    pub fn status(&self) -> QueueStatus {
        QueueStatus {
            high_priority_count: self.high_priority.len(),
            normal_priority_count: self.normal_priority.len(),
            low_priority_count: self.low_priority.len(),
            processing_count: self.processing.len(),
            total_queued: self.high_priority.len()
                + self.normal_priority.len()
                + self.low_priority.len(),
        }
    }

    /// 检查并清理超时的批次
    /// 
    /// 检查所有正在处理的批次，返回超过指定时间的批次ID列表。
    /// 同时从处理列表中移除这些超时批次。
    /// 
    /// # 参数
    /// 
    /// * `timeout_duration` - 超时阈值
    /// 
    /// # 返回值
    /// 
    /// 超时批次的ID列表
    /// 
    /// # 注意
    /// 
    /// 超时的批次会被从处理列表中移除，需要调用者决定如何处理
    pub fn check_timeouts(&mut self, timeout_duration: Duration) -> Vec<usize> {
        let now = Instant::now();
        let mut timed_out = Vec::new();

        self.processing.retain(|&batch_id, start_time| {
            if now.duration_since(*start_time) > timeout_duration {
                timed_out.push(batch_id);
                false
            } else {
                true
            }
        });

        timed_out
    }
}

impl Default for BatchQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// 批次队列状态信息
/// 
/// 提供队列中各种批次的数量统计，用于监控和调试。
/// 
/// ## 字段说明
/// 
/// - `high_priority_count`: 高优先级队列中的批次数
/// - `normal_priority_count`: 普通优先级队列中的批次数
/// - `low_priority_count`: 低优先级队列中的批次数
/// - `processing_count`: 正在处理的批次数
/// - `total_queued`: 所有队列中的批次总数（不包括正在处理的）
#[derive(Debug, Clone)]
pub struct QueueStatus {
    /// 高优先级队列中的批次数量
    pub high_priority_count: usize,
    /// 普通优先级队列中的批次数量
    pub normal_priority_count: usize,
    /// 低优先级队列中的批次数量
    pub low_priority_count: usize,
    /// 正在处理的批次数量
    pub processing_count: usize,
    /// 所有队列中的批次总数
    pub total_queued: usize,
}

/// 便利函数：使用默认配置创建优化批次
/// 
/// 为常用场景提供的快捷接口，使用默认配置的批次管理器
/// 创建优化的翻译批次。
/// 
/// # 参数
/// 
/// * `items` - 待处理的文本项列表
/// 
/// # 返回值
/// 
/// 优化后的批次列表
/// 
/// # 示例
/// 
/// ```rust
/// use crate::translation::pipeline::batch::create_optimized_batches;
/// 
/// let batches = create_optimized_batches(text_items);
/// println!("生成了 {} 个批次", batches.len());
/// ```
/// 
/// # 注意
/// 
/// 如果需要自定义配置，请直接使用 `BatchManager::new(config)`
pub fn create_optimized_batches(items: Vec<TextItem>) -> Vec<Batch> {
    let mut manager = BatchManager::default();
    manager.create_batches(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translation::collector::{TextItem, TextPriority, TextType};
    use std::sync::atomic::Ordering;

    fn create_test_text_items() -> Vec<TextItem> {
        use html5ever::parse_document;
        use html5ever::tendril::TendrilSink;
        use markup5ever_rcdom::RcDom;
        use std::io::Cursor;

        let mut input = Cursor::new("<div>test</div>");
        let dom = parse_document(RcDom::default(), Default::default())
            .from_utf8()
            .read_from(&mut input)
            .unwrap();
        let handle = dom.document.clone();

        vec![
            TextItem::content("Short text".to_string(), handle.clone(), 0),
            TextItem::content(
                "This is a medium length text for testing purposes".to_string(),
                handle.clone(),
                1,
            ),
            TextItem::content("A".repeat(500), handle.clone(), 2), // Long text
            TextItem {
                text: "Button text".to_string(),
                node: handle.clone(),
                attr_name: None,
                priority: TextPriority::High,
                text_type: TextType::Button,
                depth: 0,
                parent_tag: Some("button".to_string()),
            },
            TextItem {
                text: "Title text".to_string(),
                node: handle,
                attr_name: None,
                priority: TextPriority::Critical,
                text_type: TextType::Title,
                depth: 0,
                parent_tag: Some("h1".to_string()),
            },
        ]
    }

    #[test]
    fn test_batch_manager_creation() {
        let config = BatchManagerConfig::default();
        let manager = BatchManager::new(config.clone());

        assert_eq!(manager.config.max_batch_size, config.max_batch_size);
        assert_eq!(manager.next_batch_id.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_batch_creation_empty_input() {
        let mut manager = BatchManager::default();
        let batches = manager.create_batches(vec![]);

        assert!(batches.is_empty(), "Empty input should produce no batches");
    }

    #[test]
    fn test_batch_creation_single_item() {
        let mut manager = BatchManager::default();
        let items = vec![create_test_text_items()[0].clone()];
        let batches = manager.create_batches(items);

        assert_eq!(batches.len(), 1, "Single item should produce one batch");
        assert_eq!(batches[0].items.len(), 1, "Batch should contain one item");
        assert_eq!(
            batches[0].batch_type,
            BatchType::Single,
            "Should be Single batch type"
        );
    }

    #[test]
    fn test_batch_creation_multiple_items() {
        let mut manager = BatchManager::default();
        let items = create_test_text_items();
        let original_count = items.len();
        let batches = manager.create_batches(items);

        assert!(!batches.is_empty(), "Multiple items should produce batches");

        // Check total items preserved
        let total_items: usize = batches.iter().map(|b| b.items.len()).sum();
        assert_eq!(
            total_items, original_count,
            "All items should be preserved in batches"
        );
    }

    #[test]
    fn test_batch_manager_statistics() {
        let mut manager = BatchManager::default();
        let items = create_test_text_items();
        let item_count = items.len();

        let batches = manager.create_batches(items);
        let stats = manager.get_stats();

        assert_eq!(stats.input_items.load(Ordering::Relaxed), item_count);
        assert_eq!(stats.output_batches.load(Ordering::Relaxed), batches.len());
        assert!(
            stats.processing_time.load(Ordering::Relaxed) > 0,
            "Should have processing time"
        );
    }

    #[test]
    fn test_batch_queue_basic_operations() {
        let mut queue = BatchQueue::new();
        let items = create_test_text_items();

        // Create test batches
        let batch1 = Batch::new(1, vec![items[0].clone()], BatchType::Standard);
        let batch2 = Batch::new(2, vec![items[1].clone()], BatchType::Standard);
        let batch3 = Batch::new(3, vec![items[2].clone()], BatchType::Standard);

        queue.enqueue(batch1);
        queue.enqueue(batch2);
        queue.enqueue(batch3);

        let status = queue.status();
        assert_eq!(status.total_queued, 3, "Should have 3 batches queued");

        // Test that we can dequeue all batches
        let first = queue.dequeue().unwrap();
        let second = queue.dequeue().unwrap();
        let third = queue.dequeue().unwrap();

        // Just verify we got valid batches
        assert!(first.id > 0, "First batch should have valid ID");
        assert!(second.id > 0, "Second batch should have valid ID");
        assert!(third.id > 0, "Third batch should have valid ID");

        assert!(queue.dequeue().is_none(), "Queue should be empty");
    }
}
