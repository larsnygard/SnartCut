"""Entry point for running SnartLaser as a module: ``python -m snartlaser``."""
from __future__ import annotations

import sys


def main() -> None:
    """Launch the SnartLaser desktop application."""
    # Import here to avoid circular imports and allow headless testing
    from snartlaser.app import Application

    app = Application(sys.argv)
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
