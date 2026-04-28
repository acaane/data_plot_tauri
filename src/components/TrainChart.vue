<template>
    <div class="container" @dragover.prevent @drop.prevent>
        <h1 class="page-title">数据可视化</h1>
        <div class="header">
            <div class="controls">
                <input
                    v-model="logPath"
                    placeholder="请输入或选择文件路径"
                    class="path-input"
                />
                <button @click="selectFile" :disabled="selecting">
                    {{ selecting ? "选择中..." : "选择文件" }}
                </button>
                <button @click="loadData" :disabled="loading">
                    {{ loading ? "加载中..." : "加载数据" }}
                </button>
            </div>

            <div v-if="logType === LogType.SuidongLog" class="side-filters">
                <label class="checkbox-item">
                    <input v-model="showEast" type="checkbox" />
                    east
                </label>
                <label class="checkbox-item">
                    <input v-model="showWest" type="checkbox" />
                    west
                </label>
            </div>

            <div v-if="statusMsg" class="status">
                {{ statusMsg }}
            </div>

            <div v-if="errMsg" class="error">
                {{ errMsg }}
            </div>
        </div>

        <div class="chart-container">
            <div ref="chart" class="chart"></div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import * as echarts from 'echarts'
import { listen, TauriEvent } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

enum LogType {
    None = 0,
    PingcheLog,
    MupianLog,
    SuidongLog,
}

interface TrainInfo {
    time: string
    number: number | string
    head: number | string
    tail: number | string
    height: number | string
    pos: number | string
}

interface SuidongInfo {
    time: string
    side: string
    is_valid_distance: boolean | null
    train_distance: number | null
    is_height_updated: boolean | null
    avg_height: number | null
    raw_fields: Record<string, string>
}

const chart = ref<HTMLDivElement | null>(null)
let chartInstance: echarts.ECharts | null = null
let removeDragDropListener: null | (() => void) = null

const logPath = ref('../test_files/2026.04.28/suidong_log.log')
const selecting = ref(false)
const loading = ref(false)
const statusMsg = ref('')
const errMsg = ref('')
const showEast = ref(true)
const showWest = ref(true)
const suidongData = ref<SuidongInfo[]>([])

const timezoneOffset = 8 * 60 * 60 * 1000
const coreSuidongFields = new Set([
    'isValidDistance',
    'trainDistance',
    'isHeightUpdated',
    'avgHeight',
])

let logType = LogType.None

function parseTime(time: string): number {
    return new Date(time).getTime() - timezoneOffset
}

function formatAxisTime(value: number): string {
    const date = new Date(value)
    const yy = date.getFullYear().toString().padStart(4, '0')
    const MM = (date.getMonth() + 1).toString().padStart(2, '0')
    const dd = date.getDate().toString().padStart(2, '0')
    const hh = date.getHours().toString().padStart(2, '0')
    const mm = date.getMinutes().toString().padStart(2, '0')
    const ss = date.getSeconds().toString().padStart(2, '0')
    return `${yy}/${MM}/${dd} ${hh}:${mm}:${ss}`
}

function formatBool(value: boolean | null): string {
    if (value === null) {
        return '--'
    }

    return value ? 'true' : 'false'
}

function toSideLabel(side: string): string {
    return side.toLowerCase() === 'east' ? 'east' : 'west'
}

function ensureChart() {
    if (!chartInstance && chart.value) {
        chartInstance = echarts.init(chart.value)
    }
}

function createBaseOption(
    title: string,
    yAxis: echarts.EChartsOption['yAxis'],
    series: echarts.SeriesOption[],
    formatter: (params: any[]) => string,
): echarts.EChartsOption {
    return {
        title: {
            text: title,
            left: 'center',
            textStyle: {
                fontSize: 20,
                fontWeight: 'bold',
            },
        },
        tooltip: {
            trigger: 'axis',
            axisPointer: {
                type: 'cross',
                animation: false,
                label: {
                    backgroundColor: '#6a7985',
                },
            },
            backgroundColor: 'rgba(255, 255, 255, 0.95)',
            borderColor: '#ccc',
            borderWidth: 1,
            textStyle: {
                color: '#333',
            },
            formatter,
        },
        legend: {
            type: 'scroll',
            orient: 'vertical',
            right: 10,
            top: 60,
            bottom: 20,
            textStyle: {
                fontSize: 12,
            },
            pageIconColor: '#409eff',
            pageTextStyle: {
                color: '#666',
            },
        },
        grid: {
            left: '4%',
            right: '17%',
            bottom: '15%',
            top: '15%',
            containLabel: true,
        },
        toolbox: {
            feature: {
                dataZoom: {
                    yAxisIndex: 'none',
                    title: {
                        zoom: '区域缩放',
                        back: '缩放还原',
                    },
                },
                restore: {
                    title: '还原',
                },
                saveAsImage: {
                    title: '保存为图片',
                    backgroundColor: '#fff',
                },
            },
            right: 20,
            top: 20,
            iconStyle: {
                borderColor: '#409eff',
            },
            emphasis: {
                iconStyle: {
                    borderColor: '#66b1ff',
                },
            },
        },
        xAxis: {
            type: 'time',
            name: '时间',
            nameLocation: 'middle',
            nameGap: 30,
            boundaryGap: false,
            axisLine: {
                lineStyle: {
                    color: '#333',
                },
            },
            axisLabel: {
                formatter: (value: number) => formatAxisTime(value),
                rotate: 45,
                color: '#666',
            },
        } as any,
        yAxis,
        dataZoom: [
            {
                type: 'inside',
                start: 0,
                end: 100,
            },
            {
                start: 0,
                end: 100,
                height: 30,
                bottom: 20,
                handleStyle: {
                    color: '#409eff',
                },
                textStyle: {
                    color: '#666',
                },
            },
        ],
        series,
    }
}

function resetStateForLoad() {
    errMsg.value = ''
    suidongData.value = []
}

function checkLogType(path: string) {
    const fileName = path.split(/[\\/]/).pop() ?? ''

    if (fileName.includes('pingche_log')) {
        logType = LogType.PingcheLog
        statusMsg.value = '当前日志类型：平车机日志'
    } else if (fileName.includes('unload_log')) {
        logType = LogType.MupianLog
        statusMsg.value = '当前日志类型：木片小车日志'
    } else if (fileName.includes('suidong_log')) {
        logType = LogType.SuidongLog
        statusMsg.value = '当前日志类型：随动日志'
    } else {
        logType = LogType.None
        statusMsg.value = ''
    }
}

async function selectFile() {
    try {
        selecting.value = true
        const selected = await open({
            filters: [
                {
                    name: '日志',
                    extensions: ['*'],
                },
            ],
            multiple: false,
        })

        if (selected && typeof selected === 'string') {
            logPath.value = selected
            checkLogType(selected)
        }
    } catch (e: any) {
        errMsg.value = `选择文件失败:${e.message}`
    } finally {
        selecting.value = false
    }
}

async function loadPingcheData() {
    const data = await invoke<Record<string, TrainInfo[]>>('parse_data', {
        path: logPath.value.trim(),
    })

    if (!data || Object.keys(data).length === 0) {
        errMsg.value = '未找到有效数据'
        return
    }

    const series = Object.entries(data).map(([trainNum, records]) => {
        const sorted: [number, number, TrainInfo][] = records
            .map((record) => [
                parseTime(record.time),
                Number(record.head),
                record,
            ])
            .sort((a, b) => a[0] - b[0])

        return {
            name: `车厢:${trainNum}`,
            type: 'line',
            data: sorted,
            smooth: true,
            showSymbol: false,
            emphasis: {
                focus: 'series',
                lineStyle: {
                    width: 4,
                },
            },
        }
    })

    const option = createBaseOption(
        '车厢位置-时间曲线',
        {
            type: 'value',
            name: '位置(m)',
            nameLocation: 'middle',
            nameGap: 50,
            axisLine: {
                lineStyle: {
                    color: '#333',
                },
            },
            axisLabel: {
                color: '#666',
            },
        },
        series as echarts.SeriesOption[],
        (params: any[]): string => {
            if (!params || params.length === 0) {
                return ''
            }

            const time = new Date(params[0].value[0]).toLocaleString('zh-CN')
            let result = `
                <div style="font-weight: bold; margin-bottom: 8px; padding-bottom: 5px; border-bottom: 1px solid #eee;">
                    ${time}
                </div>
            `

            params.forEach((param) => {
                const trainNum = param.seriesName.replace('车厢:', '')
                const info = param.value[2] as TrainInfo
                result += `
                    <div style="margin: 3px 0;">
                        <span style="display: inline-block; width: 10px; height: 10px; background: ${param.color}; border-radius: 50%; margin-right: 8px;"></span>
                        <strong>车厢</strong>${trainNum}
                        <strong>车头:</strong>${Number(info.head).toFixed(2)}
                        <strong>车尾:</strong>${Number(info.tail).toFixed(2)}
                        <strong>下铲高度:</strong>${Number(info.height).toFixed(2)}
                        <strong>下铲位置:</strong>${Number(info.pos).toFixed(2)}
                    </div>
                `
            })

            return result
        },
    )

    ensureChart()
    chartInstance?.setOption(option, true)
}

async function loadMupianData() {
    const data = await invoke<any[]>('parse_mupian_data', {
        path: logPath.value.trim(),
    })

    if (!data || data.length === 0) {
        errMsg.value = '未找到有效数据'
        return
    }

    const cpuInfo = new Map<number, number>()
    const memInfo = new Map<number, [number, number]>()
    const directionInfo = new Map<number, { result: boolean; name: string }>()
    const replenishInfo = new Map<number, { result: boolean; name: string }>()

    const cpuData: [number, number][] = []
    const memData: [number, number][] = []
    const directionData: [number, number][] = []
    const replenishData: [number, number][] = []

    data.forEach((item) => {
        const time = parseTime(item.time)
        const mupianType = item.mupian_type as Record<string, any>
        const key = Object.keys(mupianType)[0]
        const value = mupianType[key]

        switch (key) {
        case 'CpuInfoTpye':
            cpuInfo.set(time, value.cpu_usage)
            memInfo.set(time, [value.mem_usage, value.mem_used])
            cpuData.push([time, value.cpu_usage / 100])
            memData.push([time, value.mem_usage / 100])
            break
        case 'DirectionMismatchInfoTpye':
            directionInfo.set(time, value)
            directionData.push([time, value.result ? 1 : 0])
            break
        case 'ReplenishInfoTpye':
            replenishInfo.set(time, value)
            replenishData.push([time, value.result ? 1 : 0])
            break
        default:
            break
        }
    })

    const option = createBaseOption(
        '木片小车CPU/内存占用-时间曲线',
        {
            type: 'value',
            name: '值',
            nameLocation: 'middle',
            nameGap: 50,
            axisLine: {
                lineStyle: {
                    color: '#333',
                },
            },
            axisLabel: {
                color: '#666',
            },
        },
        [
            {
                name: 'CPU使用率(%)',
                type: 'line',
                data: cpuData,
                smooth: true,
                showSymbol: false,
            },
            {
                name: '内存使用率(%)',
                type: 'line',
                data: memData,
                smooth: true,
                showSymbol: false,
            },
            {
                name: '方向匹配结果',
                type: 'line',
                data: directionData,
                step: 'middle',
                showSymbol: false,
            },
            {
                name: '补料完成结果',
                type: 'line',
                data: replenishData,
                step: 'middle',
                showSymbol: false,
            },
        ],
        (params: any[]): string => {
            if (!params || params.length === 0) {
                return ''
            }

            const time = params[0].value[0] as number
            const timeStr = new Date(time).toLocaleString('zh-CN')
            const cpu = cpuInfo.get(time)
            const mem = memInfo.get(time)
            const direction = directionInfo.get(time)
            const replenish = replenishInfo.get(time)

            let result = `
                <div style="font-weight: bold; margin-bottom: 8px; padding-bottom: 5px; border-bottom: 1px solid #eee;">
                    ${timeStr}
                </div>
            `

            params.forEach((param) => {
                if (param.seriesName === 'CPU使用率(%)' && cpu !== undefined) {
                    result += `
                        <div style="margin: 3px 0;">
                            <span style="display: inline-block; width: 10px; height: 10px; background: ${param.color}; border-radius: 50%; margin-right: 8px;"></span>
                            <strong>CPU使用率:</strong>${cpu.toFixed(2)}%
                        </div>
                    `
                } else if (param.seriesName === '内存使用率(%)' && mem) {
                    result += `
                        <div style="margin: 3px 0;">
                            <span style="display: inline-block; width: 10px; height: 10px; background: ${param.color}; border-radius: 50%; margin-right: 8px;"></span>
                            <strong>内存使用率:</strong>${mem[0].toFixed(2)}%
                            <strong> 内存占用:</strong>${mem[1].toFixed(2)}MB
                        </div>
                    `
                } else if (param.seriesName === '方向匹配结果' && direction) {
                    result += `
                        <div style="margin: 3px 0;">
                            <span style="display: inline-block; width: 10px; height: 10px; background: ${param.color}; border-radius: 50%; margin-right: 8px;"></span>
                            <strong>设备名:</strong>${direction.name}
                            <strong> 结果:</strong>${direction.result ? '方向匹配成功' : '方向匹配失败'}
                        </div>
                    `
                } else if (param.seriesName === '补料完成结果' && replenish) {
                    result += `
                        <div style="margin: 3px 0;">
                            <span style="display: inline-block; width: 10px; height: 10px; background: ${param.color}; border-radius: 50%; margin-right: 8px;"></span>
                            <strong>设备名:</strong>${replenish.name}
                            <strong> 结果:</strong>${replenish.result ? '补垛成功' : '补垛失败'}
                        </div>
                    `
                }
            })

            return result
        },
    )

    ensureChart()
    chartInstance?.setOption(option, true)
}

function renderSuidongChart() {
    ensureChart()

    const visibleSides = new Set<string>()
    if (showEast.value) {
        visibleSides.add('east')
    }
    if (showWest.value) {
        visibleSides.add('west')
    }

    const filteredData = suidongData.value.filter((item) => visibleSides.has(item.side))

    const seriesDefs = [
        {
            side: 'east',
            key: 'train_distance',
            label: 'east trainDistance',
            color: '#2563eb',
            yAxisIndex: 0,
        },
        {
            side: 'east',
            key: 'avg_height',
            label: 'east avgHeight',
            color: '#16a34a',
            yAxisIndex: 0,
        },
        {
            side: 'east',
            key: 'is_valid_distance',
            label: 'east isValidDistance',
            color: '#1d4ed8',
            yAxisIndex: 1,
            step: 'end',
        },
        {
            side: 'east',
            key: 'is_height_updated',
            label: 'east isHeightUpdated',
            color: '#15803d',
            yAxisIndex: 1,
            step: 'end',
        },
        {
            side: 'west',
            key: 'train_distance',
            label: 'west trainDistance',
            color: '#f59e0b',
            yAxisIndex: 0,
        },
        {
            side: 'west',
            key: 'avg_height',
            label: 'west avgHeight',
            color: '#ef4444',
            yAxisIndex: 0,
        },
        {
            side: 'west',
            key: 'is_valid_distance',
            label: 'west isValidDistance',
            color: '#d97706',
            yAxisIndex: 1,
            step: 'end',
        },
        {
            side: 'west',
            key: 'is_height_updated',
            label: 'west isHeightUpdated',
            color: '#dc2626',
            yAxisIndex: 1,
            step: 'end',
        },
    ]

    const series = seriesDefs
        .filter((def) => visibleSides.has(def.side))
        .map((def) => {
            const data = filteredData
                .filter((item) => item.side === def.side)
                .map((item) => {
                    let value: number | null = null

                    if (def.key === 'train_distance') {
                        value = item.train_distance
                    } else if (def.key === 'avg_height') {
                        value = item.avg_height
                    } else if (def.key === 'is_valid_distance') {
                        value = item.is_valid_distance === null ? null : (item.is_valid_distance ? 1 : 0)
                    } else if (def.key === 'is_height_updated') {
                        value = item.is_height_updated === null ? null : (item.is_height_updated ? 1 : 0)
                    }

                    if (value === null) {
                        return null
                    }

                    return [parseTime(item.time), value, item] as [number, number, SuidongInfo]
                })
                .filter((item): item is [number, number, SuidongInfo] => item !== null)

            return {
                name: def.label,
                type: 'line',
                yAxisIndex: def.yAxisIndex,
                data,
                smooth: def.yAxisIndex === 0,
                step: def.step,
                showSymbol: false,
                lineStyle: {
                    width: 2,
                    color: def.color,
                },
                itemStyle: {
                    color: def.color,
                },
                emphasis: {
                    focus: 'series',
                    lineStyle: {
                        width: 4,
                    },
                },
            }
        })

    const option = createBaseOption(
        '随动PLC写入信息-时间曲线',
        [
            {
                type: 'value',
                name: '距离 / 高度',
                nameLocation: 'middle',
                nameGap: 55,
                axisLine: {
                    lineStyle: {
                        color: '#333',
                    },
                },
                axisLabel: {
                    color: '#666',
                },
            },
            {
                type: 'value',
                name: '布尔状态',
                nameLocation: 'middle',
                nameGap: 55,
                min: 0,
                max: 1,
                interval: 1,
                axisLine: {
                    lineStyle: {
                        color: '#333',
                    },
                },
                axisLabel: {
                    formatter: (value: number) => (value === 1 ? 'true' : 'false'),
                    color: '#666',
                },
            },
        ],
        series as echarts.SeriesOption[],
        (params: any[]): string => {
            if (!params || params.length === 0) {
                return ''
            }

            const time = new Date(params[0].value[0]).toLocaleString('zh-CN')
            const sideMap = new Map<string, SuidongInfo>()

            params.forEach((param) => {
                const point = param.value[2] as SuidongInfo | undefined
                if (point) {
                    sideMap.set(point.side, point)
                }
            })

            let result = `
                <div style="font-weight: bold; margin-bottom: 8px; padding-bottom: 5px; border-bottom: 1px solid #eee;">
                    ${time}
                </div>
            `

            sideMap.forEach((item, side) => {
                const extraFields = Object.entries(item.raw_fields)
                    .filter(([key]) => !coreSuidongFields.has(key))
                    .map(([key, value]) => `${key}:${value}`)
                    .join(' ')

                result += `
                    <div style="margin: 6px 0; padding: 6px 0; border-bottom: 1px dashed #eee;">
                        <div><strong>${toSideLabel(side)}</strong></div>
                        <div>isValidDistance: ${formatBool(item.is_valid_distance)}</div>
                        <div>trainDistance: ${item.train_distance ?? '--'}</div>
                        <div>isHeightUpdated: ${formatBool(item.is_height_updated)}</div>
                        <div>avgHeight: ${item.avg_height ?? '--'}</div>
                        ${extraFields ? `<div>扩展字段: ${extraFields}</div>` : ''}
                    </div>
                `
            })

            return result
        },
    )

    chartInstance?.setOption(option, true)
}

async function loadSuidongData() {
    const data = await invoke<SuidongInfo[]>('parse_suidong_data', {
        path: logPath.value.trim(),
    })

    if (!data || data.length === 0) {
        errMsg.value = '未找到有效的 plc write info 数据'
        return
    }

    suidongData.value = data
    renderSuidongChart()
}

async function loadData() {
    if (!logPath.value.trim()) {
        errMsg.value = '请输入或选择文件路径'
        return
    }

    checkLogType(logPath.value)
    resetStateForLoad()
    loading.value = true

    try {
        if (logType === LogType.PingcheLog) {
            await loadPingcheData()
        } else if (logType === LogType.MupianLog) {
            await loadMupianData()
        } else if (logType === LogType.SuidongLog) {
            await loadSuidongData()
        } else {
            throw new Error('未识别的日志类型，请选择 pingche_log、unload_log 或 suidong_log 文件')
        }
    } catch (e: any) {
        console.error(`加载数据失败:${e}`)
        errMsg.value = `加载数据失败:${e.message || e}`
    } finally {
        loading.value = false
    }
}

watch([showEast, showWest], () => {
    if (logType === LogType.SuidongLog && suidongData.value.length > 0) {
        renderSuidongChart()
    }
})

onMounted(async () => {
    ensureChart()
    checkLogType(logPath.value)
    window.addEventListener('resize', handleResize)

    removeDragDropListener = await listen(TauriEvent.DRAG_DROP, (event) => {
        if (event.event !== 'tauri://drag-drop') {
            return
        }

        try {
            const payload = event.payload as { paths?: string[] }
            const paths = payload.paths ?? []
            if (paths.length > 0) {
                const path = paths[0]
                logPath.value = path
                checkLogType(path)
            }
        } catch (e: any) {
            errMsg.value = `文件拖拽失败:${e.message || e}`
        }
    })
})

onUnmounted(() => {
    window.removeEventListener('resize', handleResize)
    removeDragDropListener?.()

    if (chartInstance) {
        chartInstance.dispose()
        chartInstance = null
    }
})

function handleResize() {
    chartInstance?.resize()
}
</script>

<style scoped>
.container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 20px;
    box-sizing: border-box;
}

.page-title {
    margin: 0 0 20px 0;
    font-size: 35px;
    font-weight: 600;
    text-align: center;
    width: 100%;
    color: #333;
}

.header {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-bottom: 20px;
    padding-bottom: 15px;
    border-bottom: 1px solid #e0e0e0;
}

.controls {
    display: flex;
    gap: 10px;
    align-items: center;
    justify-content: center;
    width: 100%;
    flex-wrap: wrap;
}

.side-filters {
    display: flex;
    gap: 16px;
    align-items: center;
    margin-top: 12px;
    font-size: 14px;
    color: #333;
}

.checkbox-item {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
}

.path-input {
    width: 420px;
    max-width: 100%;
    padding: 8px 12px;
    border: 1px solid #dcdfe6;
    border-radius: 4px;
    font-size: 14px;
    transition: border-color 0.3s;
}

.path-input:focus {
    outline: none;
    border-color: #409eff;
}

button {
    padding: 8px 16px;
    background-color: #409eff;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    transition: background-color 0.3s;
}

button:hover:not(:disabled) {
    background-color: #66b1ff;
}

button:disabled {
    background-color: #a0cfff;
    cursor: not-allowed;
}

.chart-container {
    width: 100%;
    height: 100%;
    min-height: 400px;
}

.chart {
    width: 100%;
    height: 100%;
}

.status {
    margin-top: 8px;
    padding: 10px 12px;
    background-color: #ecf5ff;
    border: 1px solid #b3d8ff;
    border-radius: 4px;
    color: #409eff;
    font-size: 14px;
}

.error {
    margin-top: 8px;
    padding: 12px;
    background-color: #fef0f0;
    border: 1px solid #fbc4c4;
    border-radius: 4px;
    color: #f56c6c;
    font-size: 14px;
}
</style>
