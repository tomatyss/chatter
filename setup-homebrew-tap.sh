#!/bin/bash

# Setup script for homebrew-chatter repository
# This script will set up your homebrew tap with the correct files

echo "Setting up homebrew-chatter repository..."

# Check if we're in the right directory or need to clone
if [ ! -d "homebrew-chatter" ]; then
    echo "Cloning homebrew-chatter repository..."
    git clone https://github.com/tomatyss/homebrew-chatter.git
fi

cd homebrew-chatter

# Create Formula directory
echo "Creating Formula directory..."
mkdir -p Formula

# Copy the formula file
echo "Adding chatter formula..."
cp ../homebrew-chatter-formula.rb Formula/chatter.rb

# Copy the README
echo "Adding README..."
cp ../homebrew-chatter-readme.md README.md

# Check git status
echo "Git status:"
git status

echo ""
echo "Files created:"
echo "- Formula/chatter.rb (with correct SHA256: 985e904d2bf3f2f0350c49d3a47d1c3ed9be3f9e90ba3268833f88579cf3a5bb)"
echo "- README.md"
echo ""
echo "Next steps:"
echo "1. Review the files: git diff"
echo "2. Add files: git add ."
echo "3. Commit: git commit -m 'Add chatter formula and README'"
echo "4. Push: git push origin main"
echo ""
echo "After pushing, test with:"
echo "  brew tap tomatyss/chatter"
echo "  brew install chatter"
