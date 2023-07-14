sizes="16 32 48 96 128"
for size in $sizes; do
    echo inkscape -w $size -h $size -o icons/nadi-${size}.png icons/nadi.svg
    inkscape -w $size -h $size -o icons/nadi-${size}.png icons/nadi.svg
done

echo convert `printf "icons/nadi-%s.png " ${sizes}` icons/nadi.ico
convert `printf "icons/nadi-%s.png " ${sizes}` icons/nadi.ico
