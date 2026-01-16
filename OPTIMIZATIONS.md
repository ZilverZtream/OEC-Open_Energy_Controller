# ML Pipeline Optimizations for Raspberry Pi

## Implemented Optimizations

### 1. Memory Optimizations
- **Conservative RandomForest parameters**: n_trees=50, max_depth=10
- **Sample limiting**: Max 50k samples with automatic downsampling
- **No training sample storage**: `keep_samples: false` in RF parameters
- **Efficient matrix storage**: Row-major DenseMatrix (cache-friendly)
- **Pre-allocated vectors**: All vectors created with capacity hints

### 2. CPU Optimizations
- **Cyclical time encoding**: Sin/cos features instead of one-hot (15 features vs 100+)
- **Minimal feature set**: 15 features total (temporal + weather)
- **Batch predictions**: Vectorized operations via DenseMatrix
- **Conservative tree depth**: Prevents exponential computation

### 3. I/O Optimizations
- **Model caching**: Disk persistence prevents daily retraining
- **Streaming data fetch**: Database queries with proper indexing
- **JSON serialization**: Human-readable but still compact
- **Lazy model loading**: Only loads when ML is enabled

### 4. Training Pipeline Optimizations
- **Downsampling strategy**: Skip-based sampling preserves temporal distribution
- **Single-pass feature extraction**: Extract and normalize in one loop
- **Train/val split without shuffle**: Preserves temporal order (faster)
- **Validation on hold-out set**: No k-fold (too expensive for Pi)

### 5. Inference Optimizations
- **Feature caching**: TimeSeriesFeatures extracted once per hour
- **Fallback mechanism**: Skips ML if prediction fails (no retry storms)
- **Sanity checks**: Early rejection of invalid predictions
- **Async/await**: Non-blocking model loading and training

## Additional Optimization Opportunities

### Short-term (Easy wins)
1. **Parallel feature extraction**: Use rayon for data point processing
2. **Feature quantization**: Reduce f64 to f32 where precision isn't critical
3. **Model compression**: Use smaller RF parameters after initial training
4. **Incremental learning**: Update existing model instead of full retrain

### Medium-term (Moderate effort)
1. **Online learning**: Update model with new data points daily
2. **Feature selection**: Use feature importance to drop low-value features
3. **Model pruning**: Remove least important trees from ensemble
4. **Sparse features**: Use sparse matrix for cyclical encodings

### Long-term (Research needed)
1. **Quantized neural network**: TinyML model for Pi (< 100KB)
2. **ONNX Runtime**: Hardware-optimized inference
3. **Model distillation**: Train large model, distill to tiny one
4. **Federated learning**: Learn from multiple households securely

## Benchmark Targets (Raspberry Pi 4, 4GB RAM)

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Model training (30 days) | < 120s | TBD | ⏳ Pending |
| Model size on disk | < 5MB | TBD | ⏳ Pending |
| Inference latency | < 50ms | TBD | ⏳ Pending |
| Memory usage (training) | < 1GB | TBD | ⏳ Pending |
| Memory usage (inference) | < 100MB | TBD | ⏳ Pending |

## Configuration Tunables

Users can adjust these in the training config:

```rust
ConsumptionTrainingConfig {
    history_days: 30,        // Less data = faster training
    max_samples: 50_000,     // Lower = less memory
    n_trees: 50,             // Fewer trees = faster inference
    max_depth: Some(10),     // Shallower = less memory
    min_samples_split: 5,    // Higher = faster training
}
```

## Monitoring Recommendations

Track these metrics in production:

- Training duration (should stay < 2 min)
- Model file size (should be < 5MB)
- Prediction latency (should be < 50ms)
- Memory usage during training (should be < 1GB)
- Model R² score (should be > 0.5 for useful predictions)
