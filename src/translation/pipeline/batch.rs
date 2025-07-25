//! 批次管理器模块
//!
//! 提供智能批次分组和处理功能，优化翻译性能

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::time::{Duration, Instant};

#[cfg(feature = "translation")]
use tokio::sync::Semaphore;
#[cfg(feature = "translation")]
use tokio::time::{sleep, timeout};

use crate::translation::collector::{TextItem, TextPriority};
use crate::translation::config::{constants, TranslationConfig};
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
    /// 预估有效大小（考虑复杂度）
    pub estimated_effective_size: f32,
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
        let estimated_effective_size = Self::calculate_effective_size(&items);
        let estimated_duration = Self::estimate_duration_with_complexity(&items, estimated_effective_size);

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
    
    /// 计算有效大小（考虑复杂度）
    fn calculate_effective_size(items: &[TextItem]) -> f32 {
        items.iter().map(|item| item.effective_size()).sum()
    }

    /// 估算处理时间（基于复杂度）
    fn estimate_duration_with_complexity(items: &[TextItem], effective_size: f32) -> Duration {
        // 基于经验公式估算处理时间，考虑复杂度
        let base_time = Duration::from_millis(150); // 基础时间
        let effective_time = Duration::from_millis((effective_size as u64 * 8) / 10); // 每10个有效单位8ms
        let item_time = Duration::from_millis(items.len() as u64 * 10); // 每个项目10ms
        
        // 考虑高优先级项目的额外开销
        let priority_multiplier = items.iter()
            .map(|item| match item.priority {
                crate::translation::collector::TextPriority::Critical => 1.3,
                crate::translation::collector::TextPriority::High => 1.1,
                crate::translation::collector::TextPriority::Normal => 1.0,
                crate::translation::collector::TextPriority::Low => 0.9,
            })
            .fold(0.0, |acc, x| acc + x) / items.len() as f32;
        
        let total_duration = base_time + effective_time + item_time;
        Duration::from_millis((total_duration.as_millis() as f32 * priority_multiplier) as u64)
    }

    /// 估算处理时间（旧版本，保持向后兼容）
    fn estimate_duration(items: &[TextItem], total_chars: usize) -> Duration {
        // 基于经验公式估算处理时间
        let base_time = Duration::from_millis(100); // 基础时间
        let char_time = Duration::from_millis(total_chars as u64 / 10); // 每10个字符1ms
        let item_time = Duration::from_millis(items.len() as u64 * 5); // 每个项目5ms

        base_time + char_time + item_time
    }

    /// 检查批次是否可以合并（智能版本）
    pub fn can_merge_with(&self, other: &Batch, max_effective_size: f32) -> bool {
        // 检查优先级兼容性
        if (self.priority as i32 - other.priority as i32).abs() > 1 {
            return false;
        }

        // 检查有效大小限制（考虑复杂度）
        if self.estimated_effective_size + other.estimated_effective_size > max_effective_size {
            return false;
        }

        // 检查类型兼容性
        !matches!(
            (&self.batch_type, &other.batch_type),
            (BatchType::Single, _) | (_, BatchType::Single) | (BatchType::Large, _) | (_, BatchType::Large)
        )
    }
    
    /// 检查批次是否可以合并（旧版本，保持向后兼容）
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
            (BatchType::Single, _) | (_, BatchType::Single) | (BatchType::Large, _) | (_, BatchType::Large)
        )
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
    /// 最大有效大小（考虑复杂度）
    pub max_effective_size: f32,
    /// 最小有效大小
    pub min_effective_size: f32,
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
    /// 启用智能批次大小计算
    pub enable_smart_sizing: bool,
    /// 最大并发批次数
    pub max_concurrent_batches: usize,
    /// 批次超时时间
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
pub struct BatchManager {
    config: BatchManagerConfig,
    stats: Arc<BatchStats>,  // 使用 Arc 共享统计数据
    next_batch_id: AtomicUsize,  // 原子类型的 ID 生成器
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
            stats: Arc::new(BatchStats::default()),
            next_batch_id: AtomicUsize::new(1),
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

        self.stats.output_batches.store(optimized_batches.len(), Ordering::Relaxed);
        self.stats.add_processing_time(start_time.elapsed());

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

    /// 为特定优先级创建批次（智能版本）
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
    
    /// 智能批次创建（基于有效大小）
    fn create_smart_batches_for_priority(&mut self, items: Vec<TextItem>) -> Vec<Batch> {
        let mut batches = Vec::new();
        let mut current_batch = Vec::new();
        let mut current_effective_size = 0.0;
        let mut current_char_size = 0;
        
        for item in items {
            let item_effective_size = item.effective_size();
            let item_char_size = item.char_count();
            
            // 检查是否需要创建新批次
            let should_create_new_batch = {
                !current_batch.is_empty() && (
                    current_effective_size + item_effective_size > self.config.max_effective_size ||
                    current_char_size + item_char_size > self.config.max_batch_size
                )
            };
            
            if should_create_new_batch {
                // 创建当前批次
                if !current_batch.is_empty() {
                    let batch_type = self.determine_batch_type(
                        current_batch.len(),
                        current_char_size,
                        current_effective_size
                    );
                    let batch = Batch::new(
                        self.next_batch_id.fetch_add(1, Ordering::Relaxed),
                        std::mem::take(&mut current_batch),
                        batch_type,
                    );
                    batches.push(batch);
                    current_effective_size = 0.0;
                    current_char_size = 0;
                }
            }
            
            // 添加项目到当前批次
            current_batch.push(item);
            current_effective_size += item_effective_size;
            current_char_size += item_char_size;
        }
        
        // 处理最后一个批次
        if !current_batch.is_empty() {
            let batch_type = self.determine_batch_type(
                current_batch.len(),
                current_char_size,
                current_effective_size
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
    
    /// 传统批次创建（向后兼容）
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
    
    /// 确定批次类型（智能版本）
    fn determine_batch_type(&self, item_count: usize, char_size: usize, effective_size: f32) -> BatchType {
        if item_count == 1 {
            BatchType::Single
        } else if item_count <= self.config.small_batch_threshold {
            BatchType::Small
        } else if char_size > self.config.max_batch_size * 8 / 10 || 
                  effective_size > self.config.max_effective_size * 0.9 {
            BatchType::Large
        } else {
            BatchType::Standard
        }
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
        let would_exceed_effective = current_effective_size + item_effective_size > self.config.max_effective_size;
        // 检查字符数限制（作为后备）
        let would_exceed_chars = current_char_size + item_char_size > self.config.max_batch_size;
        
        let has_min_effective = current_effective_size >= self.config.min_effective_size;
        let has_min_chars = current_char_size >= self.config.min_batch_chars;
        let is_near_limit = current_effective_size > self.config.max_effective_size * 0.8;
        
        (would_exceed_effective || would_exceed_chars) && (has_min_effective || has_min_chars || is_near_limit)
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

/// 批次处理统计（线程安全版本）
#[derive(Debug, Default)]
pub struct BatchStats {
    pub input_items: AtomicUsize,
    pub output_batches: AtomicUsize,
    pub merged_batches: AtomicUsize,
    pub split_batches: AtomicUsize,
    pub processing_time: std::sync::atomic::AtomicU64,  // 微秒
    // 注意：平均值和分布统计需要更复杂的处理，这里先简化
    // pub average_batch_size: AtomicU32, // 可以用整数表示 
    // pub batch_size_distribution: DashMap<usize, AtomicUsize>, // 需要 dashmap 依赖
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
        self.processing_time.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
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
            let optimal_batches =
                (input_items as f32 / constants::MAX_BATCH_SIZE as f32).ceil();
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
            total_queued: self.high_priority.len()
                + self.normal_priority.len()
                + self.low_priority.len(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translation::collector::{TextItem, TextPriority, TextType};
    use std::sync::atomic::Ordering;

    fn create_test_text_items() -> Vec<TextItem> {
        use html5ever::parse_document;
        use markup5ever_rcdom::RcDom;
        use html5ever::tendril::TendrilSink;
        use std::io::Cursor;
        
        let mut input = Cursor::new("<div>test</div>");
        let dom = parse_document(RcDom::default(), Default::default())
            .from_utf8()
            .read_from(&mut input)
            .unwrap();
        let handle = dom.document.clone();
        
        vec![
            TextItem::content("Short text".to_string(), handle.clone(), 0),
            TextItem::content("This is a medium length text for testing purposes".to_string(), handle.clone(), 1),
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
        assert_eq!(batches[0].batch_type, BatchType::Single, "Should be Single batch type");
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
        assert_eq!(total_items, original_count, "All items should be preserved in batches");
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
        assert!(stats.processing_time.load(Ordering::Relaxed) > 0, "Should have processing time");
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
