#[derive(Debug)]
enum EdenShaderStage {
    ComputeBucketPreprocessing,
    ComputeMain,
    VertexRendering,
}

#[derive(Debug)]
struct ParticleBufferHandler {
    stage_one_buffer: wgpu::Buffer,
    stage_two_buffer: wgpu::Buffer,
    stage_three_buffer: wgpu::Buffer,

    stage: EdenShaderStage,
}

impl ParticleBufferHandler {
    fn new(buffer1: wgpu::Buffer, buffer2: wgpu::Buffer, buffer3: wgpu::Buffer) -> Self {
        return ParticleBufferHandler {
            stage_one_buffer: buffer1,
            stage_two_buffer: buffer2,
            stage_three_buffer: buffer3,
            stage: EdenShaderStage::ComputeBucketPreprocessing,
        };
    }

    fn set_stage(&mut self, stage: EdenShaderStage) {
        self.stage = stage;
    }

    fn get_buffers(&self) -> (&wgpu::Buffer, &wgpu::Buffer) {
        match self.stage {
            EdenShaderStage::ComputeBucketPreprocessing => {
                (&self.stage_one_buffer, &self.stage_two_buffer)
            }
            EdenShaderStage::ComputeMain => (&self.stage_two_buffer, &self.stage_three_buffer),
            EdenShaderStage::VertexRendering => (&self.stage_two_buffer, &self.stage_three_buffer),
        }
    }
}
