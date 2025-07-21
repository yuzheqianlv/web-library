//! 批次管理器模块
//!
//! 提供智能批次分组和处理功能，优化翻译性能

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(feature = "translation")]
use tokio::sync::Semaphore;
#[cfg(feature = "translation")]
use tokio::time::{sleep, timeout};

use crate::translation::collector::{TextItem, TextPriority};
use crate::translation::config::{constants, EnhancedTranslationConfig};
use crate::translation::error::{TranslationError, TranslationResult};

/// 批次信息
#[derive(Debug, Clone)]
pub struct Batch {
    /// 批次ID
    pub id: usize,
    /// 文本项列表
    pub items: Vec<TextItem>,
    /// 批次优先级
    pub priority: BatchPriority,
    /// 批次类型
    pub batch_type: BatchType,
    /// 预估字符数
    pub estimated_chars: usize,
    /// 预估处理时间
    pub estimated_duration: Duration,
    /// 创建时间
    pub created_at: Instant,
}

/// 批次优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BatchPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// 批次类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchType {
    /// 标准批次
    Standard,
    /// 小批次（特殊处理）
    Small,
    /// 大批次（需要分割）
    Large,
    /// 单项批次
    Single,
    /// 优先批次
    Priority,
}

impl Batch {
    /// 创建新批次
    pub fn new(id: usize, items: Vec<TextItem>, batch_type: BatchType) -> Self {
        let priority = Self::calculate_batch_priority(&items);
        let estimated_chars = Self::calculate_total_chars(&items);
        let estimated_duration = Self::estimate_duration(&items, estimated_chars);

        Self {
            id,
            items,
            priority,
            batch_type,
            estimated_chars,
            estimated_duration,
            created_at: Instant::now(),
        }
    }

    /// 计算批次优先级
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

    /// 计算总字符数
    fn calculate_total_chars(items: &[TextItem]) -> usize {
        items.iter().map(|item| item.char_count()).sum()
    }

    /// 估算处理时间
    fn estimate_duration(items: &[TextItem], total_chars: usize) -> Duration {
        // 基于经验公式估算处理时间
        let base_time = Duration::from_millis(100); // 基础时间
        let char_time = Duration::from_millis(total_chars as u64 / 10); // 每10个字符1ms
        let item_time = Duration::from_millis(items.len() as u64 * 5); // 每个项目5ms
        
        base_time + char_time + item_time
    }

    /// 检查批次是否可以合并
    pub fn can_merge_with(&self, other: &Batch, max_size: usize) -> bool {
        // 检查优先级兼容性
        if (self.priority as i32 - other.priority as i32).abs() > 1 {
            return false;
        }

        // 检查大小限制
        if self.estimated_chars + other.estimated_chars > max_size {
            return false;
        }

        // 检查类型兼容性
        match (&self.batch_type, &other.batch_type) {
            (BatchType::Single, _) | (_, BatchType::Single) => false,
            (BatchType::Large, _) | (_, BatchType::Large) => false,
            _ => true,
        }
    }

    /// 合并批次
    pub fn merge(mut self, other: Batch) -> Self {
        self.items.extend(other.items);
        self.priority = self.priority.max(other.priority);
        self.estimated_chars += other.estimated_chars;
        self.estimated_duration += other.estimated_duration;
        
        // 更新批次类型
        if self.estimated_chars > constants::MAX_BATCH_SIZE * 8 / 10 {
            self.batch_type = BatchType::Large;
        } else if self.items.len() <= constants::SMALL_BATCH_THRESHOLD {
            self.batch_type = BatchType::Small;
        } else {
            self.batch_type = BatchType::Standard;
        }

        self
    }

    /// 分割大批次
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

    /// 获取批次摘要
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

/// 批次管理器配置
#[derive(Debug, Clone)]
pub struct BatchManagerConfig {
    /// 最大批次大小（字符数）
    pub max_batch_size: usize,
    /// 最小批次字符数
    pub min_batch_chars: usize,
    /// 批次延迟时间
    pub batch_delay: Duration,
    /// 小批次阈值
    pub small_batch_threshold: usize,
    /// 启用优先级排序
    pub enable_priority_sorting: bool,
    /// 启用批次合并
    pub enable_batch_merging: bool,
    /// 启用智能分组
    pub enable_smart_grouping: bool,
    /// 最大并发批次数
    pub max_concurrent_batches: usize,
    /// 批次超时时间
    pub batch_timeout: Duration,
}

impl From<&EnhancedTranslationConfig> for BatchManagerConfig {
    fn from(config: &EnhancedTranslationConfig) -> Self {
        Self {
            max_batch_size: config.batch.max_batch_size,
            min_batch_chars: config.batch.min_batch_chars,
            batch_delay: Duration::from_millis(config.batch.batch_delay_ms),
            small_batch_threshold: config.batch.small_batch_threshold,
            batch_timeout: config.batch.batch_timeout,
            enable_priority_sorting: true,
            enable_batch_merging: true,
            enable_smart_grouping: true,
            max_concurrent_batches: config.performance.max_concurrent_requests,
        }
    }
}

impl Default for BatchManagerConfig {
    fn default() -> Self {
        Self {
            max_batch_size: constants::MAX_BATCH_SIZE,
            min_batch_chars: constants::DEFAULT_MIN_CHARS,
            batch_delay: Duration::from_millis(constants::BATCH_DELAY_MS),
            small_batch_threshold: constants::SMALL_BATCH_THRESHOLD,
            enable_priority_sorting: true,
            enable_batch_merging: true,
            enable_smart_grouping: true,
            max_concurrent_batches: 10,
            batch_timeout: Duration::from_secs(30),
        }
    }
}

/// 智能批次管理器
pub struct BatchManager {
    config: BatchManagerConfig,
    stats: BatchStats,
    next_batch_id: usize,
    #[cfg(feature = "translation")]
    semaphore: Arc<Semaphore>,
}

impl BatchManager {
    /// 创建新的批次管理器
    pub fn new(config: BatchManagerConfig) -> Self {
        #[cfg(feature = "translation")]
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_batches));

        Self {
            config,
            stats: BatchStats::default(),
            next_batch_id: 1,
            #[cfg(feature = "translation")]
            semaphore,
        }
    }

    /// 使用默认配置创建管理器
    pub fn default() -> Self {
        Self::new(BatchManagerConfig::default())
    }

    /// 创建优化的批次
    pub fn create_batches(&mut self, mut items: Vec<TextItem>) -> Vec<Batch> {
        let start_time = Instant::now();
        self.stats.reset();
        self.stats.input_items = items.len();

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

        self.stats.output_batches = optimized_batches.len();
        self.stats.processing_time = start_time.elapsed();

        optimized_batches
    }

    /// 最优排序算法
    fn sort_items_optimally(&self, items: &mut Vec<TextItem>) {
        items.sort_by(|a, b| {
            // 多级排序策略
            b.priority
                .cmp(&a.priority) // 优先级高的在前
                .then_with(|| a.char_count().cmp(&b.char_count())) // 短文本在前
                .then_with(|| a.depth.cmp(&b.depth)) // 浅层节点在前
        });
    }

    /// 智能批次创建
    fn create_smart_batches(&mut self, items: Vec<TextItem>) -> Vec<Batch> {
        let mut batches = Vec::new();
        let mut priority_groups = self.group_by_priority(items);

        // 按优先级处理
        for priority in [BatchPriority::Critical, BatchPriority::High, BatchPriority::Normal, BatchPriority::Low] {
            if let Some(items) = priority_groups.remove(&priority) {
                let mut priority_batches = self.create_batches_for_priority(items, priority);
                batches.append(&mut priority_batches);
            }
        }

        batches
    }

    /// 按优先级分组
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
    fn create_batches_for_priority(&mut self, items: Vec<TextItem>, _priority: BatchPriority) -> Vec<Batch> {
        let mut batches = Vec::new();
        let mut current_batch = Vec::new();
        let mut current_size = 0;

        for item in items {
            let item_size = item.char_count();

            // 超大项目单独处理
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

            // 检查是否需要开始新批次
            if self.should_start_new_batch(current_size, item_size, &current_batch) {
                self.finalize_current_batch(&mut batches, &mut current_batch, current_size);
                current_size = 0;
            }

            current_batch.push(item);
            current_size += item_size;
        }

        // 处理最后一个批次
        if !current_batch.is_empty() {
            self.finalize_current_batch(&mut batches, &mut current_batch, current_size);
        }

        batches
    }

    /// 简单批次创建（向后兼容）
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

    /// 判断是否应该开始新批次
    fn should_start_new_batch(
        &self,
        current_size: usize,
        item_size: usize,
        current_batch: &[TextItem],
    ) -> bool {
        !current_batch.is_empty()
            && (current_size + item_size > self.config.max_batch_size)
            && (current_size >= self.config.min_batch_chars || current_size > self.config.max_batch_size * 8 / 10)
    }

    /// 完成当前批次
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

    /// 优化批次（合并小批次）
    fn optimize_batches(&mut self, batches: Vec<Batch>) -> Vec<Batch> {
        if batches.len() <= 1 {
            return batches;
        }

        let mut optimized = Vec::new();
        let mut merge_candidates = VecDeque::new();

        // 将小批次和标准批次分开处理
        for batch in batches {
            match batch.batch_type {
                BatchType::Small => merge_candidates.push_back(batch),
                _ => {
                    // 先处理合并候选
                    if !merge_candidates.is_empty() {
                        optimized.extend(self.merge_small_batches(&mut merge_candidates));
                    }
                    optimized.push(batch);
                }
            }
        }

        // 处理剩余的合并候选
        if !merge_candidates.is_empty() {
            optimized.extend(self.merge_small_batches(&mut merge_candidates));
        }

        optimized
    }

    /// 合并小批次
    fn merge_small_batches(&mut self, candidates: &mut VecDeque<Batch>) -> Vec<Batch> {
        let mut merged = Vec::new();

        while let Some(mut current) = candidates.pop_front() {
            // 尝试与后续批次合并
            while let Some(next) = candidates.front() {
                if current.can_merge_with(next, self.config.max_batch_size) {
                    let next_batch = candidates.pop_front().unwrap();
                    current = current.merge(next_batch);
                    self.stats.merged_batches += 1;
                } else {
                    break;
                }
            }
            merged.push(current);
        }

        merged
    }

    /// 获取下一个批次ID
    fn next_batch_id(&mut self) -> usize {
        let id = self.next_batch_id;
        self.next_batch_id += 1;
        id
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

/// 批次处理统计
#[derive(Debug, Clone, Default)]
pub struct BatchStats {
    pub input_items: usize,
    pub output_batches: usize,
    pub merged_batches: usize,
    pub split_batches: usize,
    pub processing_time: Duration,
    pub average_batch_size: f32,
    pub batch_size_distribution: HashMap<usize, usize>,
}

impl BatchStats {
    /// 重置统计
    pub fn reset(&mut self) {
        *self = Default::default();
    }

    /// 计算平均批次大小
    pub fn calculate_average_size(&self) -> f32 {
        if self.output_batches == 0 {
            0.0
        } else {
            self.input_items as f32 / self.output_batches as f32
        }
    }

    /// 获取批次效率
    pub fn batch_efficiency(&self) -> f32 {
        if self.input_items == 0 {
            0.0
        } else {
            let optimal_batches = (self.input_items as f32 / constants::MAX_BATCH_SIZE as f32).ceil();
            optimal_batches / self.output_batches as f32
        }
    }
}

/// 批次队列管理器
pub struct BatchQueue {
    high_priority: VecDeque<Batch>,
    normal_priority: VecDeque<Batch>,
    low_priority: VecDeque<Batch>,
    processing: HashMap<usize, Instant>,
}

impl BatchQueue {
    /// 创建新的批次队列
    pub fn new() -> Self {
        Self {
            high_priority: VecDeque::new(),
            normal_priority: VecDeque::new(),
            low_priority: VecDeque::new(),
            processing: HashMap::new(),
        }
    }

    /// 添加批次到队列
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

    /// 从队列取出下一个批次
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

    /// 标记批次完成
    pub fn complete(&mut self, batch_id: usize) {
        self.processing.remove(&batch_id);
    }

    /// 获取队列状态
    pub fn status(&self) -> QueueStatus {
        QueueStatus {
            high_priority_count: self.high_priority.len(),
            normal_priority_count: self.normal_priority.len(),
            low_priority_count: self.low_priority.len(),
            processing_count: self.processing.len(),
            total_queued: self.high_priority.len() + self.normal_priority.len() + self.low_priority.len(),
        }
    }

    /// 检查处理超时
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

/// 队列状态
#[derive(Debug, Clone)]
pub struct QueueStatus {
    pub high_priority_count: usize,
    pub normal_priority_count: usize,
    pub low_priority_count: usize,
    pub processing_count: usize,
    pub total_queued: usize,
}

/// 便利函数：创建优化批次
pub fn create_optimized_batches(items: Vec<TextItem>) -> Vec<Batch> {
    let mut manager = BatchManager::default();
    manager.create_batches(items)
}