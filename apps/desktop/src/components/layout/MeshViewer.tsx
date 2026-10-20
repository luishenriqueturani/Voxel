import { Bounds, OrbitControls, useGLTF } from "@react-three/drei";
import { Canvas } from "@react-three/fiber";
import { Component, type ReactNode, Suspense, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Mesh, type Object3D } from "three";
import { applyAppearance, type MeshNode, trianglesIn } from "../../lib/mesh";

type MeshViewerProps = {
  url: string | null;
  wireframe: boolean;
  textured: boolean;
  onTriangles: (count: number) => void;
};

type BoundaryProps = {
  url: string;
  fallback: ReactNode;
  children: ReactNode;
};

type BoundaryState = { failed: boolean };

class MeshBoundary extends Component<BoundaryProps, BoundaryState> {
  override state: BoundaryState = { failed: false };

  static getDerivedStateFromError(): BoundaryState {
    return { failed: true };
  }

  override componentDidUpdate(previous: BoundaryProps): void {
    if (previous.url !== this.props.url && this.state.failed) {
      this.setState({ failed: false });
    }
  }

  override render(): ReactNode {
    return this.state.failed ? this.props.fallback : this.props.children;
  }
}

function cloneScene(scene: Object3D): Object3D {
  const next = scene.clone(true);
  next.traverse((child) => {
    if (!(child instanceof Mesh)) {
      return;
    }
    const source = child.material;
    child.material = Array.isArray(source) ? source.map((item) => item.clone()) : source.clone();
  });
  return next;
}

function LoadedMesh({
  url,
  wireframe,
  textured,
  onTriangles,
}: {
  url: string;
  wireframe: boolean;
  textured: boolean;
  onTriangles: (count: number) => void;
}) {
  const { scene } = useGLTF(url);
  const clone = useMemo(() => cloneScene(scene), [scene]);
  useEffect(() => {
    const root = clone as unknown as MeshNode;
    applyAppearance(root, wireframe, textured);
    onTriangles(trianglesIn(root));
  }, [clone, onTriangles, textured, wireframe]);
  return <primitive object={clone} />;
}

export function MeshViewer({ url, wireframe, textured, onTriangles }: MeshViewerProps) {
  const { t } = useTranslation();
  const missing = <p className="text-sm text-zinc-500">{t("viewer.meshMissing")}</p>;
  if (!url) {
    return missing;
  }
  return (
    <div
      className="relative h-full min-h-0 w-full"
      data-testid="mesh-viewer"
      role="application"
      aria-label={t("viewer.showMesh")}
    >
      <MeshBoundary url={url} fallback={missing}>
        <Canvas
          camera={{ position: [1.6, 1.1, 1.8], fov: 42 }}
          gl={{ antialias: true, alpha: true }}
        >
          <ambientLight intensity={0.7} />
          <directionalLight position={[3, 4, 2]} intensity={1.15} />
          <Suspense fallback={null}>
            <Bounds fit clip observe margin={1.2}>
              <LoadedMesh
                url={url}
                wireframe={wireframe}
                textured={textured}
                onTriangles={onTriangles}
              />
            </Bounds>
          </Suspense>
          <OrbitControls makeDefault enablePan enableZoom enableRotate />
        </Canvas>
      </MeshBoundary>
    </div>
  );
}
