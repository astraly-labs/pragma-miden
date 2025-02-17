# Pragma Publisher CLI Python Bindings

Python bindings for the Pragma Publisher CLI.

## Installation

```bash
pip install pm-publisher
```

## Configuration 


Running init for the publisher 


## Test locally

``bash
cd crates/cli/publisher
pip install -e .
python tests/python/test_publisher.py
```

You might need to run if you are using a mac (linking error)
```bash
export PYTHON_CONFIGURE_OPTS="--enable-shared"
export LDFLAGS="-L$(python3-config --prefix)/lib"
export CPPFLAGS="-I$(python3-config --prefix)/include"
```
