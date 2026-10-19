use crate::error::WorkerError;
use crate::generated::{
    Ack, CancelRequest, DecodeLatentsRequest, ImageToMeshRequest, Img2ImgRequest, InpaintRequest,
    JobOutput, LoadModelRequest, MeshPostProcessRequest, RemoveBackgroundRequest, Txt2ImgRequest,
};

/// Geração de imagem. O mesmo contrato vale para o worker real e para o falso.
pub trait ImageBackend {
    /// Carrega um modelo de imagem.
    ///
    /// # Errors
    ///
    /// Falha se o worker recusar o pedido ou estiver inacessível.
    fn load_model(&self, request: &LoadModelRequest) -> Result<Ack, WorkerError>;

    /// Gera uma imagem a partir do texto.
    ///
    /// # Errors
    ///
    /// Falha se o worker recusar o pedido ou estiver inacessível.
    fn txt2img(&self, request: &Txt2ImgRequest) -> Result<JobOutput, WorkerError>;

    /// Gera uma imagem a partir de outra imagem.
    ///
    /// # Errors
    ///
    /// Falha se o worker recusar o pedido ou estiver inacessível.
    fn img2img(&self, request: &Img2ImgRequest) -> Result<JobOutput, WorkerError>;

    /// Pinta uma região da imagem.
    ///
    /// # Errors
    ///
    /// Falha se o worker recusar o pedido ou estiver inacessível.
    fn inpaint(&self, request: &InpaintRequest) -> Result<JobOutput, WorkerError>;

    /// Decodifica latentes em uma imagem.
    ///
    /// # Errors
    ///
    /// Falha se o worker recusar o pedido ou estiver inacessível.
    fn decode_latents(&self, request: &DecodeLatentsRequest) -> Result<JobOutput, WorkerError>;

    /// Cancela um job em andamento.
    ///
    /// # Errors
    ///
    /// Falha se o worker recusar o pedido ou estiver inacessível.
    fn cancel(&self, request: &CancelRequest) -> Result<Ack, WorkerError>;

    /// Descarrega o modelo atual.
    ///
    /// # Errors
    ///
    /// Falha se o worker recusar o pedido ou estiver inacessível.
    fn unload(&self) -> Result<Ack, WorkerError>;
}

/// Geração de malha 3D a partir de uma imagem.
pub trait MeshBackend {
    /// Carrega um modelo de malha.
    ///
    /// # Errors
    ///
    /// Falha se o worker recusar o pedido ou estiver inacessível.
    fn load_model(&self, request: &LoadModelRequest) -> Result<Ack, WorkerError>;

    /// Constrói uma malha a partir de uma imagem.
    ///
    /// # Errors
    ///
    /// Falha se o worker recusar o pedido ou estiver inacessível.
    fn image_to_mesh(&self, request: &ImageToMeshRequest) -> Result<JobOutput, WorkerError>;

    /// Cancela um job em andamento.
    ///
    /// # Errors
    ///
    /// Falha se o worker recusar o pedido ou estiver inacessível.
    fn cancel(&self, request: &CancelRequest) -> Result<Ack, WorkerError>;

    /// Descarrega o modelo atual.
    ///
    /// # Errors
    ///
    /// Falha se o worker recusar o pedido ou estiver inacessível.
    fn unload(&self) -> Result<Ack, WorkerError>;
}

/// Remove o fundo de uma imagem e grava o alfa.
pub trait BackgroundRemover {
    /// Remove o fundo e grava a imagem resultante.
    ///
    /// # Errors
    ///
    /// Falha se o worker recusar o pedido ou estiver inacessível.
    fn remove_background(
        &self,
        request: &RemoveBackgroundRequest,
    ) -> Result<JobOutput, WorkerError>;
}

/// Ajustes de malha depois da geração: reduzir, limitar texturas e normalizar.
pub trait MeshPostProcess {
    /// Grava a malha ajustada.
    ///
    /// # Errors
    ///
    /// Falha se o worker recusar o pedido ou estiver inacessível.
    fn post_process(&self, request: &MeshPostProcessRequest) -> Result<JobOutput, WorkerError>;
}
