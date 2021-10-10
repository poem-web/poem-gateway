local key = KEYS[1]
local interval = tonumber(ARGV[1])
local capacity = tonumber(ARGV[2])
local timeNow = tonumber(ARGV[3])
local expire = tonumber(ARGV[4])
local currentTokens = -1
local lastFillAt = timeNow

if redis.call('exists', key) == 0 then
    currentTokens = capacity
    redis.call('hset', key, 'lastFillAt', timeNow)
else
    lastFillAt = tonumber(redis.call('hget', key, 'lastFillAt'))
    if timeNow - lastFillAt > interval then
        currentTokens = capacity
        redis.call('hset', key, 'lastFillAt', timeNow)
    else
        currentTokens = tonumber(redis.call('hget', key, 'tokens'))
    end
end

assert(currentTokens >= 0)

if expire > 0 then
    redis.call('expire', key, expire)
end

if currentTokens < 1 then
    redis.call('hset', key, 'tokens', currentTokens)
    return {false, 0}
else
    local remainingTokens = currentTokens - 1;
    redis.call('hset', key, 'tokens', remainingTokens)
    return {true, remainingTokens}
end
