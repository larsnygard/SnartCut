"""Cut-layer model.

A :class:`Layer` pairs a :class:`~snartlaser.core.types.CutSettings` object
with the set of canvas-item IDs that belong to that layer.
"""
from __future__ import annotations

from typing import List, Set

from snartlaser.core.types import CutSettings, LayerMode


# Default layer colours (cycle through these when adding new layers)
_LAYER_COLORS = [
    "#e74c3c", "#3498db", "#2ecc71", "#f39c12", "#9b59b6",
    "#1abc9c", "#e67e22", "#34495e", "#e91e63", "#00bcd4",
]


class Layer:
    """A single named cut/engrave layer.

    Attributes:
        settings: :class:`~snartlaser.core.types.CutSettings` controlling
            how this layer is processed.
        item_ids: Set of canvas-item IDs assigned to this layer.
    """

    def __init__(self, settings: CutSettings | None = None) -> None:
        self.settings: CutSettings = settings or CutSettings()
        self.item_ids: Set[str] = set()

    # ------------------------------------------------------------------
    # Convenience delegating properties
    # ------------------------------------------------------------------

    @property
    def name(self) -> str:
        return self.settings.name

    @name.setter
    def name(self, value: str) -> None:
        self.settings.name = value

    @property
    def enabled(self) -> bool:
        return self.settings.enabled

    @enabled.setter
    def enabled(self, value: bool) -> None:
        self.settings.enabled = value

    @property
    def color(self) -> str:
        return self.settings.color

    # ------------------------------------------------------------------
    # Serialisation
    # ------------------------------------------------------------------

    def to_dict(self) -> dict:
        return {
            "settings": self.settings.to_dict(),
            "item_ids": list(self.item_ids),
        }

    @classmethod
    def from_dict(cls, data: dict) -> "Layer":
        layer = cls(CutSettings.from_dict(data["settings"]))
        layer.item_ids = set(data.get("item_ids", []))
        return layer


class LayerList:
    """Ordered, mutable list of :class:`Layer` objects.

    Provides factory methods and helpers so the rest of the application
    can work with layers without knowing about internal bookkeeping.
    """

    def __init__(self) -> None:
        self._layers: List[Layer] = []

    # ------------------------------------------------------------------
    # List interface
    # ------------------------------------------------------------------

    def __len__(self) -> int:
        return len(self._layers)

    def __getitem__(self, index: int) -> Layer:
        return self._layers[index]

    def __iter__(self):
        return iter(self._layers)

    # ------------------------------------------------------------------
    # Mutation
    # ------------------------------------------------------------------

    def add(self, name: str | None = None, color: str | None = None) -> Layer:
        """Append a new :class:`Layer` and return it."""
        idx = len(self._layers)
        layer_color = color or _LAYER_COLORS[idx % len(_LAYER_COLORS)]
        settings = CutSettings(
            name=name or f"Layer {idx + 1}",
            color=layer_color,
        )
        layer = Layer(settings)
        self._layers.append(layer)
        return layer

    def remove(self, index: int) -> None:
        """Remove the layer at *index*."""
        del self._layers[index]

    def move(self, from_index: int, to_index: int) -> None:
        """Reorder layers."""
        layer = self._layers.pop(from_index)
        self._layers.insert(to_index, layer)

    def clear(self) -> None:
        self._layers.clear()

    # ------------------------------------------------------------------
    # Queries
    # ------------------------------------------------------------------

    def find_by_item(self, item_id: str) -> Layer | None:
        """Return the layer that contains *item_id*, or ``None``."""
        for layer in self._layers:
            if item_id in layer.item_ids:
                return layer
        return None

    def assign_item(self, item_id: str, layer_index: int) -> None:
        """Assign a canvas item to the layer at *layer_index*.

        Removes the item from any other layer first.
        """
        for layer in self._layers:
            layer.item_ids.discard(item_id)
        if 0 <= layer_index < len(self._layers):
            self._layers[layer_index].item_ids.add(item_id)

    @property
    def enabled_layers(self) -> List[Layer]:
        return [l for l in self._layers if l.enabled]

    # ------------------------------------------------------------------
    # Serialisation
    # ------------------------------------------------------------------

    def to_dict(self) -> list:
        return [l.to_dict() for l in self._layers]

    @classmethod
    def from_dict(cls, data: list) -> "LayerList":
        ll = cls()
        ll._layers = [Layer.from_dict(d) for d in data]
        return ll
