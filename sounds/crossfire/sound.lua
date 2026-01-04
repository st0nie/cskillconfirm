-- sound.lua for crossfire soundpack (handles variants too)
-- ctx.preset_name = full name like "crossfire" or "crossfire_v_fhd"
-- ctx.variant = nil for master, or "fhd"/"sex" for variants
--
-- Logic:
--   common.wav always from MASTER (sounds/crossfire/)
--   numbered + headshot from preset_name folder (variant or master)

function get_sounds(ctx)
    local sounds = {}
    
    -- Base path for variant-specific files (numbered, headshot)
    local base = "sounds/" .. ctx.preset_name .. "/"
    
    -- Master base for common.wav
    local master_base = "sounds/" .. ctx.master_name .. "/"
    
    -- Always play common sound from MASTER
    table.insert(sounds, master_base .. "common.wav")
    
    -- Play kill number sound (2-8) from preset folder
    if ctx.kill_count >= 2 and ctx.kill_count <= 8 then
        table.insert(sounds, base .. ctx.kill_count .. ".wav")
    end
    
    -- Play headshot sound on first headshot kill from preset folder
    if ctx.is_headshot and ctx.is_first_kill then
        table.insert(sounds, base .. "headshot.wav")
    end
    
    return sounds
end
